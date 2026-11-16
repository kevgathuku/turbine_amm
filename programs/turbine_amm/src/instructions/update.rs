use anchor_lang::prelude::*;

use crate::{error::AmmError, Pool, POOL_SEED};

#[derive(Accounts)]
pub struct Update<'info> {
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [POOL_SEED, pool.seed.to_le_bytes().as_ref()],
        bump = pool.pool_bump,
        constraint = pool.authority == Some(authority.key()) @ AmmError::InvalidAuthority,
    )]
    pub pool: Account<'info, Pool>,
}

impl<'info> Update<'info> {
    pub fn update(&mut self, locked: bool) -> Result<()> {
        self.pool.locked = locked;
        Ok(())
    }
}
