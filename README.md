# turbine_amm

[![CI](https://github.com/kevgathuku/turbine_amm/actions/workflows/ci.yml/badge.svg)](https://github.com/kevgathuku/turbine_amm/actions/workflows/ci.yml)

1. Creates a constant-product pool for two mints with a fee and treasury (`initialize`)
2. Adds liquidity, minting LP tokens proportional to the deposit (`deposit`)
3. Removes liquidity, burning LP tokens and returning the proportional share (`withdraw`)
4. Swaps one side for the other with slippage protection, sending 20% of the fee to the treasury (`swap`)
5. Locks or unlocks the pool, halting deposits and swaps while locked (`update`)

Each pool is a PDA holding its config (`seed`, both mints, `fee`, `treasury`, `locked`), with each side's liquidity in a vault ATA owned by that PDA. Swap fees stay in the vaults implicitly, except a 20% cut paid out to treasury ATAs owned by the pool's treasury key.

## Setup

Requirements: Rust 1.98.1 (see `rust-toolchain.toml`), Solana CLI, Anchor CLI, a funded localnet wallet.

Wallet for the provider (see `Anchor.toml`):

```txt
~/.config/solana/id.json
```

## Commands

```bash
# build the program
anchor build
# run all LiteSVM tests
anchor test
```

## Tests

### initialize

Initialize the pool: creates the `pool` PDA, the LP `mint_lp` PDA, and both vault ATAs, then stores the config.

Verifies:

- pool initializes with the requested seed, fee, authority, and treasury.
- Rejects fees above 100% (`FeePercentErr`).
- Rejects mismatched mint decimals (`MismatchedDecimals`).

### deposit

Deposit liquidity: moves both tokens into the vaults and mints LP tokens — verbatim amounts on first deposit, proportional shares afterwards.

Verifies:

- first deposit takes the max amounts and mints the requested LP.
- proportional deposits move exactly the computed share (boundary max passes).
- generous maxes are caps, not amounts: approval above the computed share moves nothing extra.
- Rejects deposits above the slippage bounds, on either side (`SlippageExceeded`).
- Rejects zero-amount deposits (`InvalidAmount`).
- Rejects same mint on both sides (`ConstraintDuplicateMutableAccount`).
- Rejects deposits into a locked pool (`PoolLocked`).

### withdraw

Withdraw liquidity: burns LP tokens and returns the proportional share of both vaults.

Verifies:

- burning LP returns the proportional share of each vault back to the user, verified at exact boundary minimums.
- vaults and LP supply shrink proportionally to the burn.
- Rejects withdrawals below the minimum amounts, on either side (`SlippageExceeded`).
- Rejects zero-amount withdrawals (`InvalidAmount`).
- Rejects burns above the LP supply (`InsufficientBalance`).
- Rejects withdrawals from a locked pool (`PoolLocked`).

### swap

Swap X for Y: moves the full input into the vault, pays out the fee-adjusted amount, and sends 20% of the fee to the treasury ATA.

Verifies:

- exact post-swap balances (vaults, user, treasury) against hand-computed math.
- swaps both directions (X→Y and Y→X), each paying the fee-adjusted amount with the treasury cut on the input side.
- fee stays in the pool except the treasury cut, which lands in the input side's treasury ATA.
- Rejects swaps below the minimum output, in either direction (`SlippageExceeded`).
- Rejects zero-amount swaps (`InvalidAmount`).
- Rejects swaps into a locked pool (`PoolLocked`).

### update

Lock or unlock the pool: authority-only flag flip that gates deposits and swaps.

Verifies:

- lock then unlock round-trips back to a working pool.
- locking an already-locked pool succeeds
- Rejects updates from anyone but the recorded authority (`InvalidAuthority`).
- a locked pool rejects deposits (`PoolLocked`).

## Notes

- Tests run against the freshly built `.so` — rebuild after any `src/` change
- Tests build instructions through shared helpers in `programs/turbine_amm/tests/common/mod.rs`
- Token setup (mints, ATAs, funding) goes through real Token/ATA program transactions via `litesvm-token` builders
- State-changing test setup (e.g. locking the pool) goes through real instructions, not direct account writes
