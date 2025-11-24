use crate::SwapArgs;
use crate::common_swap;
use crate::processor::swap_processor::SwapProcessor;
use crate::constants::PROFIT_SNAPSHOT_SEED;
use crate::state::profit_snapshot::ProfitSnapshot;
use anchor_lang::prelude::*;
use anchor_lang::system_program::System;
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

    // Optional: payer's WSOL and USDC token accounts for profitability check
    // If provided, they must belong to the payer; otherwise treated as zero balances
    #[account(mut)]
    pub payer_wsol_token_account: Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    #[account(mut)]
    pub payer_usdc_token_account: Option<Box<InterfaceAccount<'info, TokenAccount>>>,

    // PDA to store the "before" snapshot for end-of-tx profitability assertion
    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + ProfitSnapshot::SIZE,
        seeds = [PROFIT_SNAPSHOT_SEED, payer.key().as_ref()],
        bump
    )]
    pub profit_snapshot: Account<'info, ProfitSnapshot>,

    pub system_program: Program<'info, System>,
}

pub fn swap_handler<'a>(
    ctx: Context<'_, '_, 'a, 'a, SwapAccounts<'a>>,
    args: SwapArgs,
    order_id: u64,
) -> Result<()> {
    // Mark constant as used to satisfy compiler when referenced in attribute macros
    let _ = PROFIT_SNAPSHOT_SEED;
    // Snapshot before balances (SOL/WSOL/USDC)
    let before_snapshot = crate::utils::snapshot_wallet_balances(
        &ctx.accounts.payer,
        &mut ctx.accounts.payer_wsol_token_account,
        &mut ctx.accounts.payer_usdc_token_account,
    );

    // Persist to PDA for later assertion
    ctx.accounts.profit_snapshot.sol_lamports = before_snapshot.sol_lamports;
    ctx.accounts.profit_snapshot.wsol_amount = before_snapshot.wsol_amount;
    ctx.accounts.profit_snapshot.usdc_amount = before_snapshot.usdc_amount;

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
