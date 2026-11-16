use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{transfer, Mint, Token, TokenAccount, Transfer},
};
use constant_product_curve::{ConstantProduct, LiquidityPair, SwapResult};

use crate::{error::AmmError, Pool, MINT_LP_SEED, POOL_SEED};

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    pub mint_x: Box<Account<'info, Mint>>,
    pub mint_y: Box<Account<'info, Mint>>,

    #[account(
        has_one = mint_x,
        has_one = mint_y,
        seeds = [POOL_SEED, pool.seed.to_le_bytes().as_ref()],
        bump = pool.pool_bump,
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        seeds = [MINT_LP_SEED, pool.key().as_ref()],
        bump = pool.lp_bump,
        constraint = !pool.locked @ AmmError::PoolLocked,
    )]
    pub mint_lp: Account<'info, Mint>,

    // Pool's ATA for Token X
    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = pool,
    )]
    pub vault_x: Box<Account<'info, TokenAccount>>,

    // Pool's ATA for Token Y
    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = pool,
    )]
    pub vault_y: Box<Account<'info, TokenAccount>>,

    // User's ATA for Token X
    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = user,
    )]
    pub user_x: Box<Account<'info, TokenAccount>>,

    // User's ATA for Token Y
    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = user,
    )]
    pub user_y: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> Swap<'info> {
    pub fn swap(&mut self, is_x: bool, amount_in: u64, min_out: u64) -> Result<()> {
        require!(amount_in > 0, AmmError::InvalidAmount);

        let mut curve = ConstantProduct::init(
            self.vault_x.amount,
            self.vault_y.amount,
            self.mint_lp.supply,
            self.pool.fee,
            Some(6),
        )
        .map_err(AmmError::from)?;

        let pair = match is_x {
            true => LiquidityPair::X,
            false => LiquidityPair::Y,
        };

        let swap_result: SwapResult = curve
            .swap(pair, amount_in, min_out)
            .map_err(AmmError::from)?;

        self.deposit_tokens(is_x, swap_result.deposit)?;
        self.withdraw_tokens(is_x, swap_result.withdraw)
    }

    pub fn deposit_tokens(&mut self, is_x: bool, amount_in: u64) -> Result<()> {
        let (from, to) = match is_x {
            true => (
                self.user_x.to_account_info(),
                self.vault_x.to_account_info(),
            ),
            false => (
                self.user_y.to_account_info(),
                self.vault_y.to_account_info(),
            ),
        };

        transfer(
            CpiContext::new(
                self.token_program.key(),
                Transfer {
                    from,
                    to,
                    authority: self.user.to_account_info(),
                },
            ),
            amount_in,
        )
    }

    pub fn withdraw_tokens(&mut self, is_x: bool, amount: u64) -> Result<()> {
        let (from, to) = match is_x {
            true => (
                self.vault_y.to_account_info(),
                self.user_y.to_account_info(),
            ),
            false => (
                self.vault_x.to_account_info(),
                self.user_x.to_account_info(),
            ),
        };

        let seeds = &[
            POOL_SEED,
            &self.pool.seed.to_le_bytes()[..],
            &[self.pool.pool_bump],
        ];
        let pool_seeds = &[&seeds[..]];

        transfer(
            CpiContext::new_with_signer(
                self.token_program.key(),
                Transfer {
                    from,
                    to,
                    authority: self.pool.to_account_info(),
                },
                pool_seeds,
            ),
            amount,
        )
    }
}
