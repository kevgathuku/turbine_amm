use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

use crate::{error::AmmError, Pool, MAX_FEE_BPS, MINT_LP_SEED, POOL_SEED};

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,

    pub mint_x: Account<'info, Mint>,
    pub mint_y: Account<'info, Mint>,

    #[account(
        init,
        payer = initializer,
        seeds = [MINT_LP_SEED, pool.key.as_ref()],
        bump,
        mint::decimals = mint_x.decimals,
        mint::authority = pool,
    )]
    pub mint_lp: Account<'info, Mint>,

    #[account(
        init,
        payer = initializer,
        associated_token::mint = mint_x,
        associated_token::authority = pool,
    )]
    pub vault_x: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = initializer,
        associated_token::mint = mint_y,
        associated_token::authority = pool,
    )]
    pub vault_y: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = initializer,
        seeds = [POOL_SEED, seed.to_le_bytes().as_ref()],
        bump,
        space = Pool::DISCRIMINATOR.len() + Pool::INIT_SPACE,
    )]
    pub pool: Account<'info, Pool>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn initialize(
        &mut self,
        seed: u64,
        fee: u16,
        authority: Option<Pubkey>,
        bumps: InitializeBumps,
    ) -> Result<()> {
        require!(fee <= MAX_FEE_BPS, AmmError::FeePercentErr);
        require!(
            self.mint_x.decimals == self.mint_y.decimals,
            AmmError::MismatchedDecimals
        );

        self.pool.set_inner(Pool {
            seed,
            authority,
            mint_x: self.mint_x.key(),
            mint_y: self.mint_y.key(),
            fee,
            locked: false,
            pool_bump: bumps.pool,
            lp_bump: bumps.mint_lp,
        });
        Ok(())
    }
}
