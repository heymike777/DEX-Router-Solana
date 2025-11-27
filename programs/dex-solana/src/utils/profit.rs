use anchor_lang::prelude::*;
use anchor_lang::solana_program::program_pack::Pack;
use anchor_spl::associated_token::get_associated_token_address;
use anchor_spl::token::spl_token::state::Account as SplTokenAccount;
use anchor_spl::token_2022::spl_token_2022::state::Account as SplToken2022Account;
use anchor_spl::token_interface::TokenAccount;
use crate::{wsol_program, usdc_mint};

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

/// Find payer's WSOL and USDC token accounts from remaining_accounts by deriving ATA addresses
/// Returns AccountInfo references if found. Caller should validate they are valid token accounts.
pub fn find_token_accounts_from_remaining<'info>(
    payer: &Pubkey,
    remaining_accounts: &'info [AccountInfo<'info>],
) -> (
    Option<AccountInfo<'info>>,
    Option<AccountInfo<'info>>,
) {
    // Derive expected ATA addresses
    let wsol_ata = get_associated_token_address(payer, &wsol_program::id());
    let usdc_ata = get_associated_token_address(payer, &usdc_mint::id());

    let mut wsol_account: Option<AccountInfo<'info>> = None;
    let mut usdc_account: Option<AccountInfo<'info>> = None;

    // Search through remaining_accounts for matching addresses
    for account_info in remaining_accounts {
        if account_info.key() == wsol_ata {
            wsol_account = Some(account_info.clone());
        } else if account_info.key() == usdc_ata {
            usdc_account = Some(account_info.clone());
        }
    }

    (wsol_account, usdc_account)
}

/// Find profit snapshot PDA from remaining_accounts by deriving the PDA address
/// Returns AccountInfo if found, None otherwise
pub fn find_profit_snapshot_pda_from_remaining<'info>(
    program_id: &Pubkey,
    payer: &Pubkey,
    remaining_accounts: &'info [AccountInfo<'info>],
) -> Option<AccountInfo<'info>> {
    use crate::constants::PROFIT_SNAPSHOT_SEED;
    
    // Derive the PDA address
    let (pda_address, _bump) = Pubkey::find_program_address(
        &[PROFIT_SNAPSHOT_SEED, payer.as_ref()],
        program_id,
    );

    // Search through remaining_accounts for matching address
    for account_info in remaining_accounts {
        if account_info.key() == pda_address {
            return Some(account_info.clone());
        }
    }

    None
}

/// Snapshot wallet balances using AccountInfo references (from remaining_accounts)
pub fn snapshot_wallet_balances_from_account_info<'info>(
    payer: &AccountInfo<'info>,
    payer_wsol_token_account: Option<&AccountInfo<'info>>,
    payer_usdc_token_account: Option<&AccountInfo<'info>>,
) -> WalletSnapshot {
    let sol_lamports = payer.lamports();
    let payer_pubkey = payer.key();
    
    // Try to read WSOL balance
    let wsol_amount = if let Some(wsol_account_info) = payer_wsol_token_account {
        // Try to parse as Token or Token2022 account
        if let Ok(data) = wsol_account_info.try_borrow_data() {
            if *wsol_account_info.owner == anchor_spl::token::Token::id() && data.len() >= 165 {
                if let Ok(ta) = SplTokenAccount::unpack(&data) {
                    if ta.owner == payer_pubkey {
                        ta.amount
                    } else {
                        0
                    }
                } else {
                    0
                }
            } else if *wsol_account_info.owner == anchor_spl::token_2022::Token2022::id() && data.len() >= 165 {
                if let Ok(ta) = SplToken2022Account::unpack_from_slice(&data) {
                    if ta.owner == payer_pubkey {
                        ta.amount
                    } else {
                        0
                    }
                } else {
                    0
                }
            } else {
                0
            }
        } else {
            0
        }
    } else {
        0
    };
    
    // Try to read USDC balance
    let usdc_amount = if let Some(usdc_account_info) = payer_usdc_token_account {
        // Try to parse as Token or Token2022 account
        if let Ok(data) = usdc_account_info.try_borrow_data() {
            if *usdc_account_info.owner == anchor_spl::token::Token::id() && data.len() >= 165 {
                if let Ok(ta) = SplTokenAccount::unpack(&data) {
                    if ta.owner == payer_pubkey {
                        ta.amount
                    } else {
                        0
                    }
                } else {
                    0
                }
            } else if *usdc_account_info.owner == anchor_spl::token_2022::Token2022::id() && data.len() >= 165 {
                if let Ok(ta) = SplToken2022Account::unpack_from_slice(&data) {
                    if ta.owner == payer_pubkey {
                        ta.amount
                    } else {
                        0
                    }
                } else {
                    0
                }
            } else {
                0
            }
        } else {
            0
        }
    } else {
        0
    };

    WalletSnapshot { sol_lamports, wsol_amount, usdc_amount }
}


