mod common;

use {common::*, turbine_amm::error::AmmError};

// Withdraw half the LP
// vaults halve, user reclaims the difference, LP supply halves.
#[test]
fn test_withdraw() {
    let PoolFixture { mut svm, accounts } = setup_initialized_pool();

    let withdraw_ix = create_withdraw_tx(&accounts, LP_MINT / 2, DEPOSIT_X / 2, DEPOSIT_Y / 2);
    send(&mut svm, &[&accounts.payer], withdraw_ix).expect("withdraw should succeed");

    assert_eq!(token_balance(&svm, &accounts.vault_x), DEPOSIT_X / 2);
    assert_eq!(token_balance(&svm, &accounts.vault_y), DEPOSIT_Y / 2);
    assert_eq!(
        token_balance(&svm, &accounts.user_x),
        USER_MINT_FUNDING - DEPOSIT_X + DEPOSIT_X / 2
    );
    assert_eq!(
        token_balance(&svm, &accounts.user_y),
        USER_MINT_FUNDING - DEPOSIT_Y + DEPOSIT_Y / 2
    );
    assert_eq!(token_balance(&svm, &accounts.user_lp), LP_MINT / 2);
}

#[test]
fn test_withdraw_locked_pool_fails() {
    let PoolFixture { mut svm, accounts } = setup_initialized_pool();

    // Lock the pool through the real admin instruction.
    let lock_ix = create_update_tx(&accounts, true);
    send(&mut svm, &[&accounts.payer], lock_ix).expect("lock should succeed");

    let withdraw_ix = create_withdraw_tx(&accounts, LP_MINT / 2, DEPOSIT_X / 2, DEPOSIT_Y / 2);
    let err = send(&mut svm, &[&accounts.payer], withdraw_ix).unwrap_err();

    let expected = AmmError::PoolLocked.to_string();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains(&expected),
        "expected {expected}, got: {err_str}"
    );
}

#[test]
fn test_withdraw_zero_amount_fails() {
    let PoolFixture { mut svm, accounts } = setup_initialized_pool();

    let withdraw_ix = create_withdraw_tx(&accounts, 0, 0, 0);
    let err = send(&mut svm, &[&accounts.payer], withdraw_ix).unwrap_err();

    let expected = AmmError::InvalidAmount.to_string();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains(&expected),
        "expected {expected}, got: {err_str}"
    );
}

#[test]
fn test_withdraw_slippage_exceeded_fails() {
    let PoolFixture { mut svm, accounts } = setup_initialized_pool();

    // min_x one unit above the proportional share.
    let withdraw_ix = create_withdraw_tx(&accounts, LP_MINT / 2, DEPOSIT_X / 2 + 1, DEPOSIT_Y / 2);
    let err = send(&mut svm, &[&accounts.payer], withdraw_ix).unwrap_err();

    let expected = AmmError::SlippageExceeded.to_string();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains(&expected),
        "expected {expected}, got: {err_str}"
    );
}

#[test]
fn test_withdraw_slippage_y_exceeded_fails() {
    let PoolFixture { mut svm, accounts } = setup_initialized_pool();

    // min_y one unit above the proportional share.
    let withdraw_ix = create_withdraw_tx(&accounts, LP_MINT / 2, DEPOSIT_X / 2, DEPOSIT_Y / 2 + 1);
    let err = send(&mut svm, &[&accounts.payer], withdraw_ix).unwrap_err();

    let expected = AmmError::SlippageExceeded.to_string();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains(&expected),
        "expected {expected}, got: {err_str}"
    );
}

#[test]
fn test_withdraw_more_than_supply_fails() {
    let PoolFixture { mut svm, accounts } = setup_initialized_pool();

    let withdraw_ix = create_withdraw_tx(&accounts, u64::MAX, 0, 0);
    let err = send(&mut svm, &[&accounts.payer], withdraw_ix).unwrap_err();

    let expected = AmmError::InsufficientBalance.to_string();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains(&expected),
        "expected {expected}, got: {err_str}"
    );
}
