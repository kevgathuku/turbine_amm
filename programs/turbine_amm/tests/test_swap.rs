mod common;

use {
    anchor_spl::associated_token::get_associated_token_address,
    common::*,
    turbine_amm::{constants::TREASURY_CUT_BPS, error::AmmError},
};

// Swap 10M X for Y: full amount in, fee-adjusted amount out
// x1 = 200,000,000 (vault_x after first deposit)
// y1 = 300,000,000 (vault_y after first deposit)
// k  = x1 * y1 = 60,000,000,000,000,000
// a2 = in - fee = 10,000,000 - 30,000 = 9,970,000
// y2 = k / (x1 + a2) = 285,755,107
// out = y1 - y2 = 14,244,893
const SWAP_IN: u64 = 10_000_000;
const FEE: u64 = SWAP_IN * FEE_BPS as u64 / 10_000;
const A2: u64 = SWAP_IN - FEE;
const K: u64 = DEPOSIT_X * DEPOSIT_Y;
const Y2: u64 = K / (DEPOSIT_X + A2);
const SWAP_OUT: u64 = DEPOSIT_Y - Y2;

// Treasury cut: 20% of the 30,000-unit fee, paid in the input token.
const TREASURY_CUT: u64 = FEE * TREASURY_CUT_BPS as u64 / 10_000;

#[test]
fn test_swap_x_for_y() {
    let PoolFixture { mut svm, accounts } = setup_initialized_pool();

    let swap_ix = create_swap_tx(&accounts, true, SWAP_IN, SWAP_OUT);
    send(&mut svm, &[&accounts.payer], swap_ix).expect("swap should succeed");

    // 80% of fee stays in the pool
    assert_eq!(
        token_balance(&svm, &accounts.vault_x),
        DEPOSIT_X + SWAP_IN - TREASURY_CUT
    );
    assert_eq!(token_balance(&svm, &accounts.vault_y), DEPOSIT_Y - SWAP_OUT);
    assert_eq!(
        token_balance(
            &svm,
            &get_associated_token_address(&accounts.treasury, &accounts.mint_x)
        ),
        TREASURY_CUT
    );
    assert_eq!(
        token_balance(
            &svm,
            &get_associated_token_address(&accounts.treasury, &accounts.mint_y)
        ),
        0
    );
    assert_eq!(
        token_balance(&svm, &accounts.user_x),
        USER_MINT_FUNDING - DEPOSIT_X - SWAP_IN
    );
    assert_eq!(
        token_balance(&svm, &accounts.user_y),
        USER_MINT_FUNDING - DEPOSIT_Y + SWAP_OUT
    );
}

#[test]
fn test_swap_locked_pool_fails() {
    let PoolFixture { mut svm, accounts } = setup_initialized_pool();

    // Lock the pool through the real admin instruction.
    let lock_ix = create_update_tx(&accounts, true);
    send(&mut svm, &[&accounts.payer], lock_ix).expect("lock should succeed");

    let swap_ix = create_swap_tx(&accounts, true, SWAP_IN, SWAP_OUT);
    let err = send(&mut svm, &[&accounts.payer], swap_ix).unwrap_err();

    let expected = AmmError::PoolLocked.to_string();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains(&expected),
        "expected {expected}, got: {err_str}"
    );
}

#[test]
fn test_swap_zero_amount_fails() {
    let PoolFixture { mut svm, accounts } = setup_initialized_pool();

    let swap_ix = create_swap_tx(&accounts, true, 0, 0);
    let err = send(&mut svm, &[&accounts.payer], swap_ix).unwrap_err();

    let expected = AmmError::InvalidAmount.to_string();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains(&expected),
        "expected {expected}, got: {err_str}"
    );
}

#[test]
fn test_swap_slippage_exceeded_fails() {
    let PoolFixture { mut svm, accounts } = setup_initialized_pool();

    let swap_ix = create_swap_tx(&accounts, true, SWAP_IN, SWAP_OUT + 1);
    let err = send(&mut svm, &[&accounts.payer], swap_ix).unwrap_err();

    let expected = AmmError::SlippageExceeded.to_string();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains(&expected),
        "expected {expected}, got: {err_str}"
    );
}

// Mirror image: swap 10M Y for X. Both legs haircut identically
// a2 = 10M * (10000 - 30) / 10000 = 9,970,000
// y2 = y1 + b = 300M + 10M = 310,000,000 (full input lands in the vault)
// x2 = k / (y1 + a2) = 193,567,119, where k = x1 * y1
// out = x1 - x2 = 6,432,881
const SWAP_Y_IN: u64 = 10_000_000;
const SWAP_X_OUT: u64 = 6_432_881;

#[test]
fn test_swap_y_for_x() {
    let PoolFixture { mut svm, accounts } = setup_initialized_pool();

    let swap_ix = create_swap_tx(&accounts, false, SWAP_Y_IN, SWAP_X_OUT);
    send(&mut svm, &[&accounts.payer], swap_ix).expect("swap should succeed");

    // Full input lands in vault_y; the treasury cut comes off the Y side.
    assert_eq!(
        token_balance(&svm, &accounts.vault_y),
        DEPOSIT_Y + SWAP_Y_IN - TREASURY_CUT
    );
    assert_eq!(
        token_balance(&svm, &accounts.vault_x),
        DEPOSIT_X - SWAP_X_OUT
    );

    assert_eq!(
        token_balance(&svm, &accounts.user_y),
        USER_MINT_FUNDING - DEPOSIT_Y - SWAP_Y_IN
    );
    assert_eq!(
        token_balance(&svm, &accounts.user_x),
        USER_MINT_FUNDING - DEPOSIT_X + SWAP_X_OUT
    );

    // Treasury collected on the input (Y) side only.
    let treasury = accounts.treasury;
    assert_eq!(
        token_balance(
            &svm,
            &get_associated_token_address(&treasury, &accounts.mint_y)
        ),
        TREASURY_CUT
    );
    assert_eq!(
        token_balance(
            &svm,
            &get_associated_token_address(&treasury, &accounts.mint_x)
        ),
        0
    );
}
