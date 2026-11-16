use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Pool {
    pub seed: u64,                 // Seed to be able to create different pools / configs
    pub authority: Option<Pubkey>, // If we want an authority to lock the config account
    pub mint_x: Pubkey,            // Token X
    pub mint_y: Pubkey,            // Token Y
    pub fee: u16,                  // Swap fee in basis points
    pub locked: bool,              // If the pool is locked
    pub treasury: Pubkey,          // Wallet receiving the treasury cut of swap fees
    pub pool_bump: u8,             // Bump seed for the liquidity pool account
    pub lp_bump: u8,               // Bump seed for the LP token
}
