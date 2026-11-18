use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{burn, transfer, Burn, Mint, Token, TokenAccount, Transfer},
};
use constant_product_curve::ConstantProduct;

use crate::{error::AmmError, Pool, MINT_LP_SEED, POOL_SEED};

#[derive(Accounts)]
pub struct Withdraw<'info> {
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
    pub pool: Box<Account<'info, Pool>>,

    // mint_lp is mutable because the supply of minted LP tokens will reduce
    #[account(
        mut,
        seeds = [MINT_LP_SEED, pool.key().as_ref()],
        bump = pool.lp_bump,
    )]
    pub mint_lp: Box<Account<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = pool,
    )]
    pub vault_x: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = pool,
    )]
    pub vault_y: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = user,
    )]
    pub user_x: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = user,
    )]
    pub user_y: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_lp,
        associated_token::authority = user,
    )]
    pub user_lp: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    // every associated_token::mint/authority constraint i
    // expands to code that references that associated_token_program
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> Withdraw<'info> {
    pub fn withdraw(
        &mut self,
        amount: u64, // Amount of LP tokens that the user wants to "burn"
        min_x: u64,  // Minimum amount of token X that the user wants to receive
        min_y: u64,  // Minimum amount of token Y that the user wants to receive
    ) -> Result<()> {
        require!(!self.pool.locked, AmmError::PoolLocked);
        require_neq!(amount, 0, AmmError::InvalidAmount);
        require!(
            amount <= self.mint_lp.supply,
            AmmError::InsufficientBalance
        );

        let mut curve = ConstantProduct::init(
            self.vault_x.amount,
            self.vault_y.amount,
            self.mint_lp.supply,
            self.pool.fee,
            None, // 10^6 precision, the library default
        )
        .map_err(AmmError::from)?;
        let amounts = curve
            .withdraw_liquidity(amount, min_x, min_y)
            .map_err(AmmError::from)?;
        let (x, y) = (amounts.withdraw_x, amounts.withdraw_y);

        self.burn_lp_tokens(amount)?;
        self.withdraw_tokens(true, x)?;
        self.withdraw_tokens(false, y)
    }

    pub fn withdraw_tokens(&self, is_x: bool, amount: u64) -> Result<()> {
        let (from, to) = match is_x {
            true => (
                self.vault_x.to_account_info(),
                self.user_x.to_account_info(),
            ),
            false => (
                self.vault_y.to_account_info(),
                self.user_y.to_account_info(),
            ),
        };

        transfer(
            CpiContext::new_with_signer(
                self.token_program.key(),
                Transfer {
                    from,
                    to,
                    authority: self.pool.to_account_info(),
                },
                &[&[
                    POOL_SEED,
                    &self.pool.seed.to_le_bytes(),
                    &[self.pool.pool_bump],
                ]],
            ),
            amount,
        )
    }

    pub fn burn_lp_tokens(&self, amount: u64) -> Result<()> {
        burn(
            CpiContext::new(
                self.token_program.key(),
                Burn {
                    mint: self.mint_lp.to_account_info(),
                    from: self.user_lp.to_account_info(),
                    authority: self.user.to_account_info(),
                },
            ),
            amount,
        )
    }
}
