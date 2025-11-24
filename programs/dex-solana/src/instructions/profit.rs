use anchor_lang::prelude::*;
use anchor_lang::prelude::InterfaceAccount;
use anchor_spl::token_interface::TokenAccount;
use crate::error::ErrorCode;
use crate::utils::{snapshot_wallet_balances, compute_profit_lamports, WalletSnapshot};
use crate::constants::PROFIT_SNAPSHOT_SEED;
use crate::state::profit_snapshot::ProfitSnapshot;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct ProfitAssertArgs {
    pub before_sol_lamports: u64,
    pub before_wsol_amount: u64,
    pub before_usdc_amount: u64,
}

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
    /// Optional PDA snapshot created earlier (e.g., by swap), will be closed to payer if provided
    #[account(
        mut,
        seeds = [PROFIT_SNAPSHOT_SEED, payer.key().as_ref()],
        bump,
        close = payer
    )]
    pub profit_snapshot: Option<Account<'info, ProfitSnapshot>>,
}

/// Assert that the entire transaction (up to this final instruction) is profitable for `payer`.
/// Caller provides the "before" snapshot values as args; the instruction reads the "after" balances on-chain.
/// This should be used as the LAST instruction in the transaction to include all prior effects (e.g., Jito tips).
pub fn profit_assert_handler<'a>(
    ctx: Context<'_, '_, 'a, 'a, ProfitAssertAccounts<'a>>,
    args: ProfitAssertArgs,
) -> Result<()> {
    // Mark constant as used to satisfy compiler when referenced in attribute macros
    let _ = PROFIT_SNAPSHOT_SEED;
    let after = snapshot_wallet_balances(
        &ctx.accounts.payer,
        &mut ctx.accounts.payer_wsol_token_account,
        &mut ctx.accounts.payer_usdc_token_account,
    );
    let before = if let Some(snapshot) = &ctx.accounts.profit_snapshot {
        WalletSnapshot {
            sol_lamports: snapshot.sol_lamports,
            wsol_amount: snapshot.wsol_amount,
            usdc_amount: snapshot.usdc_amount,
        }
    } else {
        WalletSnapshot {
            sol_lamports: args.before_sol_lamports,
            wsol_amount: args.before_wsol_amount,
            usdc_amount: args.before_usdc_amount,
        }
    };
    let profit = compute_profit_lamports(&before, &after);
    require!(profit >= 0, ErrorCode::UnprofitableTransaction);
    Ok(())
}


