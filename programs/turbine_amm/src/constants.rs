use anchor_lang::prelude::*;

#[constant]
pub const MINT_LP_SEED: &[u8] = b"lp";

#[constant]
pub const POOL_SEED: &[u8] = b"pool";

/// Maximum fee in basis points (10_000 bps = 100%).
#[constant]
pub const MAX_FEE_BPS: u16 = 10_000;

/// Share of each swap fee paid out to the treasury, in basis points of the fee.
/// 2_000 = 20% to the treasury, the rest stays in the pool.
#[constant]
pub const TREASURY_CUT_BPS: u16 = 2_000;
