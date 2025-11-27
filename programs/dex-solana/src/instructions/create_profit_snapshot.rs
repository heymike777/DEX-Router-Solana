use anchor_lang::prelude::*;
use crate::state::profit_snapshot::ProfitSnapshot;
use crate::utils::{
    find_token_accounts_from_remaining,
    snapshot_wallet_balances_from_account_info,
    init_profit_snapshot_if_needed,
};
use crate::constants::PROFIT_SNAPSHOT_SEED;

#[derive(Accounts)]
pub struct CreateProfitSnapshotAccounts<'info> {
    /// The wallet whose balances we are snapshotting
    #[account(mut)]
    pub payer: Signer<'info>,
    
    /// PDA to store the "before" snapshot for end-of-tx profitability assertion
    /// Will be initialized if it doesn't exist, otherwise reused
    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + ProfitSnapshot::SIZE,
        seeds = [PROFIT_SNAPSHOT_SEED, payer.key().as_ref()],
        bump,
    )]
    pub profit_snapshot: Account<'info, ProfitSnapshot>,
    
    /// System program for account initialization
    pub system_program: Program<'info, System>,
}

/// Create/initialize the profit snapshot PDA and store current "before" balances.
/// This should be called BEFORE the swap instruction to capture pre-swap state.
pub fn create_profit_snapshot_handler<'a>(
    ctx: Context<'_, '_, 'a, 'a, CreateProfitSnapshotAccounts<'a>>,
) -> Result<()> {
    msg!("=== Create Profit Snapshot ===");
    msg!("Payer: {}", ctx.accounts.payer.key());
    msg!("Snapshot PDA: {}", ctx.accounts.profit_snapshot.key());
    
    // Find WSOL/USDC token accounts from remaining_accounts if present
    let (wsol_account_info, usdc_account_info) = find_token_accounts_from_remaining(
        ctx.accounts.payer.key,
        ctx.remaining_accounts,
    );
    
    // Capture "before" snapshot
    let before = snapshot_wallet_balances_from_account_info(
        &ctx.accounts.payer.to_account_info(),
        wsol_account_info.as_ref(),
        usdc_account_info.as_ref(),
    );
    
    msg!("Captured before snapshot: SOL={}, WSOL={}, USDC={}", 
         before.sol_lamports, before.wsol_amount, before.usdc_amount);
    
    // Write snapshot data (Anchor's init_if_needed already initialized the account)
    ctx.accounts.profit_snapshot.sol_lamports = before.sol_lamports;
    ctx.accounts.profit_snapshot.wsol_amount = before.wsol_amount;
    ctx.accounts.profit_snapshot.usdc_amount = before.usdc_amount;
    
    msg!("Snapshot data written successfully");
    
    Ok(())
}
