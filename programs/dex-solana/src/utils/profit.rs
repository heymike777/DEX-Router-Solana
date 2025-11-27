use anchor_lang::prelude::*;
use anchor_lang::solana_program::program_pack::Pack;
use anchor_spl::associated_token::get_associated_token_address;
use anchor_spl::token::spl_token::state::Account as SplTokenAccount;
use anchor_spl::token_2022::spl_token_2022::state::Account as SplToken2022Account;
use anchor_spl::token_interface::TokenAccount;
use crate::{wsol_program, usdc_mint};
use crate::state::profit_snapshot::ProfitSnapshot;

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
/// Returns AccountInfo and bump if found, None otherwise
pub fn find_profit_snapshot_pda_from_remaining<'info>(
    program_id: &Pubkey,
    payer: &Pubkey,
    remaining_accounts: &'info [AccountInfo<'info>],
) -> Option<(AccountInfo<'info>, u8)> {
    use crate::constants::PROFIT_SNAPSHOT_SEED;
    
    // Derive the PDA address
    let (pda_address, bump) = Pubkey::find_program_address(
        &[PROFIT_SNAPSHOT_SEED, payer.as_ref()],
        program_id,
    );

    // Search through remaining_accounts for matching address
    for account_info in remaining_accounts {
        if account_info.key() == pda_address {
            return Some((account_info.clone(), bump));
        }
    }

    None
}

/// Initialize profit snapshot PDA if not already initialized
/// Returns true if account was initialized, false if it was already initialized
pub fn init_profit_snapshot_if_needed<'info>(
    snapshot_account: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    program_id: &Pubkey,
    system_program: &AccountInfo<'info>,
    bump: u8,
) -> Result<bool> {
    use crate::constants::PROFIT_SNAPSHOT_SEED;
    use anchor_lang::solana_program::{system_instruction, program::invoke_signed};
    use anchor_lang::Discriminator;
    
    // Check if account is already initialized (has data with discriminator)
    let required_size = 8 + ProfitSnapshot::SIZE; // 8 bytes discriminator + data
    
    msg!("Initializing snapshot account: {}", snapshot_account.key());
    msg!("Account data_len: {}, required: {}", snapshot_account.data_len(), required_size);
    msg!("Account owner: {}", snapshot_account.owner);
    msg!("Account lamports: {}", snapshot_account.lamports());
    
    // Check if account has correct discriminator
    if snapshot_account.data_len() >= required_size {
        if let Ok(data) = snapshot_account.try_borrow_data() {
            if data.len() >= 8 {
                let discriminator = ProfitSnapshot::DISCRIMINATOR;
                if &data[0..8] == &discriminator[..] {
                    // Account is already initialized
                    msg!("Snapshot account already initialized");
                    return Ok(false);
                }
            }
        }
    }
    
    msg!("Snapshot account needs initialization");

    // Account is not initialized, initialize it
    let space = required_size as u64;
    let rent = anchor_lang::solana_program::rent::Rent::get()?;
    let rent_lamports = rent.minimum_balance(required_size);

    // Create seeds for PDA signing
    let seeds: &[&[u8]] = &[PROFIT_SNAPSHOT_SEED, payer.key.as_ref(), &[bump]];
    let signer_seeds: &[&[&[u8]]] = &[seeds];

    use anchor_lang::solana_program::system_program;
    
    // Ensure account has enough rent (transfer creates account if it doesn't exist)
    let current_lamports = snapshot_account.lamports();
    if current_lamports < rent_lamports {
        let additional_lamports = rent_lamports - current_lamports;
        anchor_lang::solana_program::program::invoke(
            &system_instruction::transfer(payer.key, snapshot_account.key, additional_lamports),
            &[payer.clone(), snapshot_account.clone(), system_program.clone()],
        )?;
    }

    // Refresh account info after transfer (account might have been created)
    // Check if account needs to be allocated and assigned
    if snapshot_account.owner == &system_program::id() {
        // Account exists but is owned by system program - allocate and assign
        if snapshot_account.data_len() < required_size {
            let allocate_ix = system_instruction::allocate(snapshot_account.key, space);
            invoke_signed(
                &allocate_ix,
                &[snapshot_account.clone()],
                signer_seeds,
            )?;
        }
        
        let assign_ix = system_instruction::assign(snapshot_account.key, program_id);
        invoke_signed(
            &assign_ix,
            &[snapshot_account.clone()],
            signer_seeds,
        )?;
    } else if snapshot_account.owner != program_id {
        // Account is owned by something else - this shouldn't happen
        return Err(anchor_lang::error::ErrorCode::ConstraintOwner.into());
    }
    
    // Ensure account has correct size (should always be true after above, but double-check)
    if snapshot_account.data_len() < required_size {
        return Err(anchor_lang::error::ErrorCode::AccountNotEnoughKeys.into());
    }

    // Initialize the account data with discriminator and zero values
    let mut account_data = snapshot_account.try_borrow_mut_data()?;
    
    // Always ensure discriminator is set (account was just allocated/assigned)
    if account_data.len() >= 8 {
        let discriminator = ProfitSnapshot::DISCRIMINATOR;
        // Initialize all bytes to 0 first
        account_data.fill(0);
        // Write discriminator
        account_data[0..8].copy_from_slice(&discriminator);
        msg!("Initialized snapshot account with discriminator");
    }

    Ok(true)
}

/// Find system_program from remaining_accounts
pub fn find_system_program_from_remaining<'info>(
    remaining_accounts: &'info [AccountInfo<'info>],
) -> Option<AccountInfo<'info>> {
    use anchor_lang::solana_program::system_program;
    
    for account_info in remaining_accounts {
        if account_info.key() == system_program::id() {
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
        // Check if account exists and is initialized (has lamports and data)
        if wsol_account_info.lamports() == 0 || wsol_account_info.data_is_empty() {
            0 // Account doesn't exist or is not initialized - treat as zero balance
        } else if let Ok(data) = wsol_account_info.try_borrow_data() {
            // Try to parse as Token or Token2022 account
            if data.len() >= 165 {
                if *wsol_account_info.owner == anchor_spl::token::Token::id() {
                    if let Ok(ta) = SplTokenAccount::unpack(&data) {
                        if ta.owner == payer_pubkey {
                            ta.amount
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                } else if *wsol_account_info.owner == anchor_spl::token_2022::Token2022::id() {
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
                    0 // Not a token account
                }
            } else {
                0 // Account data too small (not initialized)
            }
        } else {
            0 // Failed to borrow data
        }
    } else {
        0 // Account not provided
    };
    
    // Try to read USDC balance
    let usdc_amount = if let Some(usdc_account_info) = payer_usdc_token_account {
        // Check if account exists and is initialized (has lamports and data)
        if usdc_account_info.lamports() == 0 || usdc_account_info.data_is_empty() {
            0 // Account doesn't exist or is not initialized - treat as zero balance
        } else if let Ok(data) = usdc_account_info.try_borrow_data() {
            // Try to parse as Token or Token2022 account
            if data.len() >= 165 {
                if *usdc_account_info.owner == anchor_spl::token::Token::id() {
                    if let Ok(ta) = SplTokenAccount::unpack(&data) {
                        if ta.owner == payer_pubkey {
                            ta.amount
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                } else if *usdc_account_info.owner == anchor_spl::token_2022::Token2022::id() {
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
                    0 // Not a token account
                }
            } else {
                0 // Account data too small (not initialized)
            }
        } else {
            0 // Failed to borrow data
        }
    } else {
        0 // Account not provided
    };

    WalletSnapshot { sol_lamports, wsol_amount, usdc_amount }
}

/// Read ProfitSnapshot from AccountInfo (used when account is found from remaining_accounts)
pub fn read_profit_snapshot_from_account_info<'info>(
    snapshot_account: &AccountInfo<'info>,
    program_id: &Pubkey,
) -> Result<ProfitSnapshot> {
    use anchor_lang::Discriminator;
    
    // Check account has correct size
    let required_size = 8 + ProfitSnapshot::SIZE; // 8 bytes discriminator + data
    
    // Check if account is initialized (has data and is owned by our program)
    msg!("Checking snapshot account size:");
    msg!("Account data_len: {}", snapshot_account.data_len());
    msg!("Required size: {}", required_size);
    
    if snapshot_account.data_len() < required_size {
        msg!("ERROR: Snapshot account doesn't have enough data (not initialized)");
        msg!("Account: {}", snapshot_account.key());
        msg!("Data length: {}", snapshot_account.data_len());
        msg!("Required length: {}", required_size);
        return Err(anchor_lang::error::ErrorCode::AccountNotInitialized.into());
    }
    
    // Check account is owned by program
    // Log details for debugging
    use anchor_lang::solana_program::system_program;
    msg!("Checking snapshot account ownership:");
    msg!("Account: {}", snapshot_account.key());
    msg!("Account owner: {}", snapshot_account.owner);
    msg!("Expected program_id: {}", program_id);
    msg!("Account lamports: {}", snapshot_account.lamports());
    
    if snapshot_account.owner == &system_program::id() {
        msg!("ERROR: Snapshot account is not initialized (owned by system program)");
        return Err(anchor_lang::error::ErrorCode::AccountNotInitialized.into());
    }
    
    if snapshot_account.owner != program_id {
        msg!("ERROR: Snapshot account owner mismatch!");
        msg!("Account: {}", snapshot_account.key());
        msg!("Actual owner: {}", snapshot_account.owner);
        msg!("Expected owner: {}", program_id);
        return Err(anchor_lang::error::ErrorCode::ConstraintOwner.into());
    }
    
    // Borrow and verify discriminator
    let data = snapshot_account.try_borrow_data()?;
    let discriminator = ProfitSnapshot::DISCRIMINATOR;
    
    // Check if discriminator matches
    if data.len() < 8 || &data[0..8] != &discriminator[..] {
        return Err(anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch.into());
    }
    
    // Deserialize the snapshot data (skip 8-byte discriminator)
    let sol_lamports = u64::from_le_bytes(data[8..16].try_into().unwrap());
    let wsol_amount = u64::from_le_bytes(data[16..24].try_into().unwrap());
    let usdc_amount = u64::from_le_bytes(data[24..32].try_into().unwrap());
    
    Ok(ProfitSnapshot {
        sol_lamports,
        wsol_amount,
        usdc_amount,
    })
}


