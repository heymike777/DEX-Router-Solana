use crate::SwapArgs;
use crate::common_swap;
use crate::processor::swap_processor::SwapProcessor;
use crate::state::profit_snapshot::ProfitSnapshot;
use crate::utils::{find_token_accounts_from_remaining, find_profit_snapshot_pda_from_remaining, snapshot_wallet_balances_from_account_info};
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
    if let Some(snapshot_account_info) = find_profit_snapshot_pda_from_remaining(
        ctx.program_id,
        ctx.accounts.payer.key,
        ctx.remaining_accounts,
    ) {
        // Check if account is writable and has correct size
        if snapshot_account_info.is_writable && snapshot_account_info.data_len() >= 8 + ProfitSnapshot::SIZE {
            // Try to write to the account if it's already initialized
            if let Ok(mut snapshot_data) = snapshot_account_info.try_borrow_mut_data() {
                // Check if account is initialized (has discriminator)
                // Anchor accounts have an 8-byte discriminator at the start
                let min_data_size = 8 + ProfitSnapshot::SIZE;
                if snapshot_data.len() >= min_data_size {
                    // Create "before" snapshot
                    let before = snapshot_wallet_balances_from_account_info(
                        &ctx.accounts.payer.to_account_info(),
                        wsol_account_info.as_ref(),
                        usdc_account_info.as_ref(),
                    );

                    // Write snapshot data (skip 8-byte discriminator)
                    snapshot_data[8..16].copy_from_slice(&before.sol_lamports.to_le_bytes());
                    snapshot_data[16..24].copy_from_slice(&before.wsol_amount.to_le_bytes());
                    snapshot_data[24..32].copy_from_slice(&before.usdc_amount.to_le_bytes());
                }
            }
        }
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
