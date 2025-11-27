use crate::SwapArgs;
use crate::common_swap;
use crate::processor::swap_processor::SwapProcessor;
use crate::state::profit_snapshot::ProfitSnapshot;
use crate::utils::{
    find_token_accounts_from_remaining, 
    find_profit_snapshot_pda_from_remaining, 
    snapshot_wallet_balances_from_account_info,
    init_profit_snapshot_if_needed,
    find_system_program_from_remaining,
};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount};

#[derive(Accounts)]
pub struct SwapAccounts<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        token::mint = source_mint,
        token::authority = payer,
    )]
    pub source_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = destination_mint,
    )]
    pub destination_token_account: InterfaceAccount<'info, TokenAccount>,

    pub source_mint: InterfaceAccount<'info, Mint>,

    pub destination_mint: InterfaceAccount<'info, Mint>,
}

pub fn swap_handler<'a>(
    ctx: Context<'_, '_, 'a, 'a, SwapAccounts<'a>>,
    args: SwapArgs,
    order_id: u64,
) -> Result<()> {
    // Automatically find WSOL/USDC token accounts from remaining_accounts if present
    let (wsol_account_info, usdc_account_info) = find_token_accounts_from_remaining(
        ctx.accounts.payer.key,
        ctx.remaining_accounts,
    );

    // Automatically find profit_snapshot PDA from remaining_accounts if present
    if let Some((snapshot_account_info, bump)) = find_profit_snapshot_pda_from_remaining(
        ctx.program_id,
        ctx.accounts.payer.key,
        ctx.remaining_accounts,
    ) {
        msg!("Found snapshot PDA in swap: {}", snapshot_account_info.key());
        msg!("Snapshot is writable: {}", snapshot_account_info.is_writable);
        msg!("Snapshot data_len: {}", snapshot_account_info.data_len());
        
        // Check if account is writable
        if snapshot_account_info.is_writable {
            // Find system_program for initialization if needed
            if let Some(system_program_info) = find_system_program_from_remaining(ctx.remaining_accounts) {
                msg!("Found system_program, initializing snapshot if needed...");
                // Initialize account if not already initialized
                let was_initialized = init_profit_snapshot_if_needed(
                    &snapshot_account_info,
                    &ctx.accounts.payer.to_account_info(),
                    ctx.program_id,
                    &system_program_info,
                    bump,
                )?;
                msg!("Snapshot initialization result: was_initialized = {}", was_initialized);
            } else {
                msg!("WARNING: system_program not found in remaining_accounts - cannot initialize snapshot");
            }

            // Write snapshot data (account should be initialized now)
            let min_data_size = 8 + ProfitSnapshot::SIZE;
            msg!("Attempting to write snapshot data. Account data_len: {}, required: {}", 
                 snapshot_account_info.data_len(), min_data_size);
            
            // Check if account is properly initialized before writing
            if snapshot_account_info.data_len() < min_data_size {
                msg!("ERROR: Snapshot account not properly initialized!");
                msg!("Account data_len: {}, required: {}", snapshot_account_info.data_len(), min_data_size);
                msg!("Make sure snapshot PDA and system_program are in swap's remaining_accounts");
                // Continue with swap anyway - snapshot is optional
            } else if let Ok(mut snapshot_data) = snapshot_account_info.try_borrow_mut_data() {
                // Create "before" snapshot
                let before = snapshot_wallet_balances_from_account_info(
                    &ctx.accounts.payer.to_account_info(),
                    wsol_account_info.as_ref(),
                    usdc_account_info.as_ref(),
                );

                msg!("Writing before snapshot: SOL={}, WSOL={}, USDC={}", 
                     before.sol_lamports, before.wsol_amount, before.usdc_amount);

                // Ensure discriminator is written first
                use anchor_lang::Discriminator;
                let discriminator = ProfitSnapshot::DISCRIMINATOR;
                snapshot_data[0..8].copy_from_slice(&discriminator);
                
                // Write snapshot data (skip 8-byte discriminator)
                snapshot_data[8..16].copy_from_slice(&before.sol_lamports.to_le_bytes());
                snapshot_data[16..24].copy_from_slice(&before.wsol_amount.to_le_bytes());
                snapshot_data[24..32].copy_from_slice(&before.usdc_amount.to_le_bytes());
                
                msg!("Snapshot data written successfully");
            } else {
                msg!("ERROR: Failed to borrow mutable data from snapshot account");
            }
        } else {
            msg!("WARNING: Snapshot account is not writable - cannot write snapshot data");
        }
    } else {
        msg!("Snapshot PDA not found in remaining_accounts - skipping snapshot");
    }

    common_swap(
        &SwapProcessor,
        &ctx.accounts.payer,
        &ctx.accounts.payer,
        None,
        &mut ctx.accounts.source_token_account,
        &mut ctx.accounts.destination_token_account,
        &ctx.accounts.source_mint,
        &ctx.accounts.destination_mint,
        &None,
        &mut None,
        &mut None,
        &None,
        &None,
        &None,
        &None,
        ctx.remaining_accounts,
        args,
        order_id,
        None,
        None,
        None,
    )?;

    Ok(())
}
