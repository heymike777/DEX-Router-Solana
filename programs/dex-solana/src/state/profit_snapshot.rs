use anchor_lang::prelude::*;

#[account]
pub struct ProfitSnapshot {
    pub sol_lamports: u64,
    pub wsol_amount: u64,
    pub usdc_amount: u64,
}

impl ProfitSnapshot {
    pub const SIZE: usize = 8 + 8 + 8; // three u64
}


