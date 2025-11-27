use anchor_lang::prelude::*;
use anchor_lang::prelude::InterfaceAccount;
use anchor_lang::solana_program::sysvar::instructions::ID as INSTRUCTIONS_SYSVAR_ID;
use anchor_spl::token_interface::TokenAccount;
use crate::error::ErrorCode;
use crate::utils::{
    compute_profit_lamports, 
    WalletSnapshot,
    snapshot_wallet_balances_from_account_info,
    find_token_accounts_from_remaining,
    find_profit_snapshot_pda_from_remaining,
    read_profit_snapshot_from_account_info,
};
use crate::constants::{PROFIT_SNAPSHOT_SEED, SIGNATURE_FEE, DEFAULT_COMPUTE_UNIT_LIMIT, compute_budget_program};
use crate::state::profit_snapshot::ProfitSnapshot;

#[derive(Accounts)]
pub struct ProfitAssertAccounts<'info> {
    /// The wallet whose profitability we are checking
    pub payer: Signer<'info>,
    /// Optional PDA snapshot created earlier (e.g., by swap), contains "before" balances
    /// CHECK: Can be found from remaining_accounts if not provided. Will be closed to payer after reading (rent refunded)
    #[account(mut)]
    pub profit_snapshot: Option<UncheckedAccount<'info>>,
    /// CHECK: Solana Instructions Sysvar for reading compute budget instructions
    #[account(address = INSTRUCTIONS_SYSVAR_ID)]
    pub instructions_sysvar: UncheckedAccount<'info>,
}

/// Assert that the entire transaction (up to this final instruction) is profitable for `payer`.
/// Reads "before" balances from the required profit_snapshot PDA (automatically created by swap).
/// Transaction fees are automatically calculated and deducted.
/// This should be used as the LAST instruction in the transaction to include all prior effects (e.g., Jito tips).
pub fn profit_assert_handler<'a>(
    ctx: Context<'_, '_, 'a, 'a, ProfitAssertAccounts<'a>>,
) -> Result<()> {
    // Mark constant as used to satisfy compiler when referenced in attribute macros
    let _ = PROFIT_SNAPSHOT_SEED;
    
    // Log which wallet is being checked
    msg!("=== Profit Assert - Wallet Check ===");
    msg!("Checking profit for payer wallet: {}", ctx.accounts.payer.key());
    
    // Debug: Check if snapshot account is provided
    msg!("profit_snapshot provided in struct: {}", ctx.accounts.profit_snapshot.is_some());
    if let Some(ref snapshot) = ctx.accounts.profit_snapshot {
        msg!("Snapshot account from struct: {}", snapshot.key());
    }
    msg!("Number of remaining_accounts: {}", ctx.remaining_accounts.len());
    
    // Find token accounts from remaining_accounts (they're always optional)
    let (wsol_account_info, usdc_account_info) = find_token_accounts_from_remaining(
        ctx.accounts.payer.key,
        ctx.remaining_accounts,
    );
    
    // Read "after" balances on-chain (handles uninitialized accounts gracefully)
    let after = snapshot_wallet_balances_from_account_info(
        &ctx.accounts.payer.to_account_info(),
        wsol_account_info.as_ref(),
        usdc_account_info.as_ref(),
    );
    
    // Find profit snapshot PDA from remaining_accounts if not provided in struct
    let snapshot_account_info = if let Some(ref snapshot) = ctx.accounts.profit_snapshot {
        msg!("Using snapshot account from struct: {}", snapshot.key());
        Some(snapshot.to_account_info())
    } else {
        msg!("Snapshot not in struct, searching remaining_accounts...");
        find_profit_snapshot_pda_from_remaining(
            ctx.program_id,
            ctx.accounts.payer.key,
            ctx.remaining_accounts,
        ).map(|(account_info, _bump)| {
            msg!("Found snapshot in remaining_accounts: {}", account_info.key());
            account_info
        })
    };
    
    // Read "before" balances from snapshot PDA
    let snapshot_data = snapshot_account_info
        .ok_or_else(|| {
            msg!("ERROR: Snapshot account not found in struct or remaining_accounts!");
            anchor_lang::error::ErrorCode::AccountNotEnoughKeys
        })?;
    
    msg!("About to read snapshot from account: {}", snapshot_data.key());
    
    // Try to read snapshot, and if not initialized, use zero balances as fallback
    let before = match read_profit_snapshot_from_account_info(&snapshot_data, ctx.program_id) {
        Ok(snapshot) => {
            msg!("Snapshot found and initialized - using stored 'before' balances");
            WalletSnapshot {
                sol_lamports: snapshot.sol_lamports,
                wsol_amount: snapshot.wsol_amount,
                usdc_amount: snapshot.usdc_amount,
            }
        }
        Err(e) => {
            // Check if error is because account is not initialized
            if snapshot_data.data_len() == 0 {
                msg!("WARNING: Snapshot account is not initialized!");
                msg!("This means the swap instruction didn't capture 'before' balances.");
                msg!("Falling back to using zero balances as baseline (checking if final balance > 0).");
                msg!("For accurate profit checking, ensure swap instruction includes snapshot PDA and system_program in remaining_accounts.");
                WalletSnapshot {
                    sol_lamports: 0,
                    wsol_amount: 0,
                    usdc_amount: 0,
                }
            } else {
                // Some other error - fail
                return Err(e);
            }
        }
    };
    
    // Close the snapshot account if it's writable (refund rent to payer)
    if snapshot_data.is_writable {
        // Transfer lamports from snapshot account back to payer (close account)
        let rent_to_refund = snapshot_data.lamports();
        **snapshot_data.lamports.borrow_mut() = 0;
        **ctx.accounts.payer.lamports.borrow_mut() = ctx.accounts.payer
            .lamports()
            .checked_add(rent_to_refund)
            .ok_or(ErrorCode::CalculationError)?;
    }
    // Log before snapshot values
    msg!("=== Profit Assert - Before Snapshot ===");
    msg!("before_sol_lamports: {}", before.sol_lamports);
    msg!("before_wsol_amount: {}", before.wsol_amount);
    msg!("before_usdc_amount: {}", before.usdc_amount);
    
    // Log after snapshot values
    msg!("=== Profit Assert - After Snapshot ===");
    msg!("after_sol_lamports: {}", after.sol_lamports);
    msg!("after_wsol_amount: {}", after.wsol_amount);
    msg!("after_usdc_amount: {}", after.usdc_amount);
    
    // Calculate deltas
    let delta_sol = after.sol_lamports as i128 - before.sol_lamports as i128;
    let delta_wsol = after.wsol_amount as i128 - before.wsol_amount as i128;
    let delta_usdc = after.usdc_amount as i128 - before.usdc_amount as i128;
    
    // Log deltas
    msg!("=== Profit Assert - Balance Deltas ===");
    msg!("delta_sol_lamports: {}", delta_sol);
    msg!("delta_wsol_amount: {}", delta_wsol);
    msg!("delta_usdc_amount: {}", delta_usdc);
    
    // Calculate profit
    let profit = compute_profit_lamports(&before, &after);
    msg!("=== Profit Assert - Profit Calculation ===");
    msg!("profit_before_fees_lamports: {}", profit);
    msg!("profit_formula: delta_sol + delta_wsol + (delta_usdc * 5)");
    msg!("profit_calculation: {} + {} + ({} * 5) = {}", delta_sol, delta_wsol, delta_usdc, profit);
    
    // Calculate transaction fees from compute budget instructions
    let (transaction_fees, fee_breakdown) = compute_transaction_fees(&ctx.accounts.instructions_sysvar)?;
    
    // Log transaction fee breakdown
    msg!("=== Profit Assert - Transaction Fees ===");
    msg!("compute_unit_limit: {}", fee_breakdown.compute_unit_limit);
    msg!("compute_unit_price_microlamports: {}", fee_breakdown.compute_unit_price_microlamports);
    msg!("priority_fee_lamports: {}", fee_breakdown.priority_fee_lamports);
    msg!("signature_fee_lamports: {}", fee_breakdown.signature_fee_lamports);
    msg!("total_transaction_fees_lamports: {}", transaction_fees);
    
    // Subtract transaction fees (priority fees, compute units, signature fees) from profit
    let profit_after_fees = profit - transaction_fees as i128;
    msg!("=== Profit Assert - Final Calculation ===");
    msg!("profit_after_fees_lamports: {}", profit_after_fees);
    msg!("profit_after_fees_formula: profit_before_fees - total_transaction_fees");
    msg!("profit_after_fees_calculation: {} - {} = {}", profit, transaction_fees, profit_after_fees);
    
    require!(profit_after_fees >= 0, ErrorCode::UnprofitableTransaction);
    msg!("=== Profit Assert - Result ===");
    msg!("✅ Transaction is PROFITABLE (profit_after_fees >= 0)");
    Ok(())
}

/// Fee breakdown structure for detailed logging
struct FeeBreakdown {
    compute_unit_limit: u32,
    compute_unit_price_microlamports: u64,
    priority_fee_lamports: u64,
    signature_fee_lamports: u64,
}

/// Compute transaction fees by reading compute budget program instructions from the instructions sysvar.
/// Returns total fees in lamports (signature fee + priority fees) and detailed breakdown.
fn compute_transaction_fees(instruction_sysvar_account_info: &AccountInfo) -> Result<(u64, FeeBreakdown)> {
    use anchor_lang::solana_program::sysvar::instructions::load_instruction_at_checked;
    
    let mut compute_unit_limit = Some(DEFAULT_COMPUTE_UNIT_LIMIT);
    let mut compute_unit_price = None;
    let mut i = 0;
    
    // Iterate through all instructions to find compute budget program instructions
    loop {
        match load_instruction_at_checked(i, instruction_sysvar_account_info) {
            Ok(instruction) => {
                if instruction.program_id == compute_budget_program::id() {
                    // Parse SetComputeUnitLimit instruction (0x02)
                    if instruction.data.len() >= 5 && instruction.data[0] == 0x02 {
                        let units = u32::from_le_bytes(
                            instruction.data[1..5].try_into().map_err(|_| ErrorCode::CalculationError)?
                        );
                        compute_unit_limit = Some(units);
                    }
                    // Parse SetComputeUnitPrice instruction (0x03)
                    else if instruction.data.len() >= 9 && instruction.data[0] == 0x03 {
                        let price = u64::from_le_bytes(
                            instruction.data[1..9].try_into().map_err(|_| ErrorCode::CalculationError)?
                        );
                        compute_unit_price = Some(price);
                    }
                }
            }
            Err(_) => {
                break;
            }
        }
        i += 1;
    }

    // Calculate total fee: (compute_units * price_per_unit) / 1_000_000 + signature_fee
    let (total_fee, breakdown) = if let (Some(units), Some(price)) = (compute_unit_limit, compute_unit_price) {
        let priority_fee = u64::from(units)
            .checked_mul(price)
            .ok_or(ErrorCode::CalculationError)?
            .checked_div(1_000_000)
            .ok_or(ErrorCode::CalculationError)?;
        
        let total = priority_fee
            .checked_add(SIGNATURE_FEE)
            .ok_or(ErrorCode::CalculationError)?;
        
        (
            total,
            FeeBreakdown {
                compute_unit_limit: units,
                compute_unit_price_microlamports: price,
                priority_fee_lamports: priority_fee,
                signature_fee_lamports: SIGNATURE_FEE,
            },
        )
    } else {
        // No priority fee set, only signature fee
        (
            SIGNATURE_FEE,
            FeeBreakdown {
                compute_unit_limit: DEFAULT_COMPUTE_UNIT_LIMIT,
                compute_unit_price_microlamports: 0,
                priority_fee_lamports: 0,
                signature_fee_lamports: SIGNATURE_FEE,
            },
        )
    };
    
    Ok((total_fee, breakdown))
}


