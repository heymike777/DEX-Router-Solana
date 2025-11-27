use anchor_lang::prelude::*;
use anchor_lang::prelude::InterfaceAccount;
use anchor_lang::solana_program::sysvar::instructions::ID as INSTRUCTIONS_SYSVAR_ID;
use anchor_spl::token_interface::TokenAccount;
use crate::error::ErrorCode;
use crate::utils::{snapshot_wallet_balances, compute_profit_lamports, WalletSnapshot};
use crate::constants::{PROFIT_SNAPSHOT_SEED, SIGNATURE_FEE, DEFAULT_COMPUTE_UNIT_LIMIT, compute_budget_program};
use crate::state::profit_snapshot::ProfitSnapshot;

#[derive(Accounts)]
pub struct ProfitAssertAccounts<'info> {
    /// The wallet whose profitability we are checking
    pub payer: Signer<'info>,
    /// Optional WSOL token account of the payer (So111... mint)
    #[account(mut)]
    pub payer_wsol_token_account: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    /// Optional USDC token account of the payer (EPjF... mint)
    #[account(mut)]
    pub payer_usdc_token_account: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    /// Required PDA snapshot created earlier (e.g., by swap), contains "before" balances
    /// Will be closed to payer after reading (rent refunded)
    #[account(
        mut,
        seeds = [PROFIT_SNAPSHOT_SEED, payer.key().as_ref()],
        bump,
        close = payer
    )]
    pub profit_snapshot: Account<'info, ProfitSnapshot>,
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
    
    // Read "after" balances on-chain
    let after = snapshot_wallet_balances(
        &ctx.accounts.payer,
        &mut ctx.accounts.payer_wsol_token_account,
        &mut ctx.accounts.payer_usdc_token_account,
    );
    
    // Read "before" balances from snapshot PDA (required)
    let before = WalletSnapshot {
        sol_lamports: ctx.accounts.profit_snapshot.sol_lamports,
        wsol_amount: ctx.accounts.profit_snapshot.wsol_amount,
        usdc_amount: ctx.accounts.profit_snapshot.usdc_amount,
    };
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


