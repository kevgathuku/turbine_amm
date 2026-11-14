use anchor_lang::prelude::*;

#[constant]
pub const MINT_LP_SEED: &[u8] = b"lp";

#[constant]
pub const POOL_SEED: &[u8] = b"pool";

/// Maximum fee in basis points (10_000 bps = 100%).
#[constant]
pub const MAX_FEE_BPS: u16 = 10_000;
