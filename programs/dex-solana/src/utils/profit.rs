use anchor_lang::prelude::*;
use anchor_spl::token_interface::{InterfaceAccount, TokenAccount};

/// Snapshot of payer's relevant balances
pub struct WalletSnapshot {
    pub sol_lamports: u64,
    pub wsol_amount: u64,  // 9 decimals (lamports-equivalent)
    pub usdc_amount: u64,  // 6 decimals
}

/// Take a snapshot of payer SOL/WSOL/USDC balances.
/// If WSOL/USDC accounts are None or closed, balances are treated as 0.
pub fn snapshot_wallet_balances<'info>(
    payer: &AccountInfo<'info>,
    payer_wsol_token_account: &mut Option<Box<InterfaceAccount<'info, TokenAccount>>>,
    payer_usdc_token_account: &mut Option<Box<InterfaceAccount<'info, TokenAccount>>>,
) -> WalletSnapshot {
    let sol_lamports = payer.lamports();
    let wsol_amount = match payer_wsol_token_account.as_mut() {
        Some(acc) => {
            if acc.get_lamports() > 0 {
                // Ensure the cached data is refreshed
                let _ = acc.reload();
                acc.amount
            } else {
                0
            }
        }
        None => 0,
    };
    let usdc_amount = match payer_usdc_token_account.as_mut() {
        Some(acc) => {
            if acc.get_lamports() > 0 {
                // Ensure the cached data is refreshed
                let _ = acc.reload();
                acc.amount
            } else {
                0
            }
        }
        None => 0,
    };

    WalletSnapshot { sol_lamports, wsol_amount, usdc_amount }
}

/// Compute profit (in lamports) between two snapshots using:
/// profit = ΔSOL + ΔWSOL + ΔUSDC * 5
/// Explanation:
/// - SOL and WSOL use 9 decimals (lamports)
/// - USDC uses 6 decimals; assuming 1 SOL ~ 200 USDC, then 1 USDC (1e6) ≈ 5,000,000 lamports
///   Therefore, 1 micro USDC ≈ 5 lamports, so ΔUSDC_base_units * 5 gives lamports
pub fn compute_profit_lamports(before: &WalletSnapshot, after: &WalletSnapshot) -> i128 {
    let delta_sol = after.sol_lamports as i128 - before.sol_lamports as i128;
    let delta_wsol = after.wsol_amount as i128 - before.wsol_amount as i128;
    let delta_usdc = after.usdc_amount as i128 - before.usdc_amount as i128;
    delta_sol + delta_wsol + (delta_usdc * 5)
}


