mod common;

use {anchor_lang::error::ErrorCode, common::*, turbine_amm::error::AmmError};

#[test]
fn test_deposit() {
    let PoolFixture { mut svm, accounts } = setup();

    let init_ix = create_initialize_tx(&accounts, 30);
    send(&mut svm, &[&accounts.payer], init_ix).expect("initialize should succeed");

    let deposit_ix = create_deposit_tx(&accounts, LP_MINT, DEPOSIT_X, DEPOSIT_Y);
    send(&mut svm, &[&accounts.payer], deposit_ix).expect("deposit should succeed");

    assert_eq!(token_balance(&svm, &accounts.vault_x), DEPOSIT_X);
    assert_eq!(token_balance(&svm, &accounts.vault_y), DEPOSIT_Y);
    assert_eq!(
        token_balance(&svm, &accounts.user_x),
        USER_MINT_FUNDING - DEPOSIT_X
    );
    assert_eq!(
        token_balance(&svm, &accounts.user_y),
        USER_MINT_FUNDING - DEPOSIT_Y
    );
    assert_eq!(token_balance(&svm, &accounts.user_lp), LP_MINT);
}

#[test]
fn test_deposit_same_mint_fails() {
    let PoolFixture { mut svm, accounts } = setup_initialized_pool();

    // Same mint for X and Y: the Y-accounts alias their X counterparts.
    let bad_accounts = PoolAccounts {
        mint_y: accounts.mint_x,
        vault_y: accounts.vault_x,
        user_y: accounts.user_x,
        ..accounts
    };
    let ix = create_deposit_tx(&bad_accounts, LP_MINT, DEPOSIT_X, DEPOSIT_Y);
    let err = send(&mut svm, &[&bad_accounts.payer], ix).unwrap_err();

    let expected = ErrorCode::ConstraintDuplicateMutableAccount.to_string();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains(&expected),
        "expected {expected}, got: {err_str}"
    );
}

#[test]
fn test_deposit_locked_pool_fails() {
    let PoolFixture { mut svm, accounts } = setup_initialized_pool();

    // Lock the pool through the real admin instruction.
    let lock_ix = create_update_tx(&accounts, true);
    send(&mut svm, &[&accounts.payer], lock_ix).expect("lock should succeed");

    let deposit_ix = create_deposit_tx(&accounts, LP_MINT, DEPOSIT_X, DEPOSIT_Y);
    let err = send(&mut svm, &[&accounts.payer], deposit_ix).unwrap_err();

    let expected = AmmError::PoolLocked.to_string();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains(&expected),
        "expected {expected}, got: {err_str}"
    );
}

#[test]
fn test_deposit_proportional_succeeds() {
    let PoolFixture { mut svm, accounts } = setup_initialized_pool();

    // Second deposit of half the LP supply takes exactly half of each vault.
    let deposit_ix = create_deposit_tx(&accounts, LP_MINT / 2, DEPOSIT_X / 2, DEPOSIT_Y / 2);
    send(&mut svm, &[&accounts.payer], deposit_ix).expect("proportional deposit should succeed");

    assert_eq!(
        token_balance(&svm, &accounts.vault_x),
        DEPOSIT_X + DEPOSIT_X / 2
    );
    assert_eq!(
        token_balance(&svm, &accounts.vault_y),
        DEPOSIT_Y + DEPOSIT_Y / 2
    );
    assert_eq!(
        token_balance(&svm, &accounts.user_x),
        USER_MINT_FUNDING - DEPOSIT_X - DEPOSIT_X / 2
    );
    assert_eq!(
        token_balance(&svm, &accounts.user_y),
        USER_MINT_FUNDING - DEPOSIT_Y - DEPOSIT_Y / 2
    );
    assert_eq!(
        token_balance(&svm, &accounts.user_lp),
        LP_MINT + LP_MINT / 2
    );
}

#[test]
fn test_deposit_slippage_x_exceeded_fails() {
    let PoolFixture { mut svm, accounts } = setup_initialized_pool();

    // max_x one unit below the proportional share.
    let deposit_ix = create_deposit_tx(&accounts, LP_MINT / 2, DEPOSIT_X / 2 - 1, DEPOSIT_Y / 2);
    let err = send(&mut svm, &[&accounts.payer], deposit_ix).unwrap_err();

    let expected = AmmError::SlippageExceeded.to_string();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains(&expected),
        "expected {expected}, got: {err_str}"
    );
}

#[test]
fn test_deposit_slippage_y_exceeded_fails() {
    let PoolFixture { mut svm, accounts } = setup_initialized_pool();

    // max_y one unit below the proportional share.
    let deposit_ix = create_deposit_tx(&accounts, LP_MINT / 2, DEPOSIT_X, DEPOSIT_Y / 2 - 1);
    let err = send(&mut svm, &[&accounts.payer], deposit_ix).unwrap_err();

    let expected = AmmError::SlippageExceeded.to_string();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains(&expected),
        "expected {expected}, got: {err_str}"
    );
}

#[test]
fn test_deposit_zero_amount_fails() {
    let PoolFixture { mut svm, accounts } = setup_initialized_pool();

    let deposit_ix = create_deposit_tx(&accounts, 0, DEPOSIT_X, DEPOSIT_Y);
    let err = send(&mut svm, &[&accounts.payer], deposit_ix).unwrap_err();

    let expected = AmmError::InvalidAmount.to_string();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains(&expected),
        "expected {expected}, got: {err_str}"
    );
}

#[test]
fn test_deposit_generous_max_takes_computed_amounts() {
    let PoolFixture { mut svm, accounts } = setup_initialized_pool();

    // Maxes far above the proportional share: only the computed amounts move.
    let deposit_ix = create_deposit_tx(&accounts, LP_MINT / 2, DEPOSIT_X, DEPOSIT_Y);
    send(&mut svm, &[&accounts.payer], deposit_ix).expect("generous deposit should succeed");

    assert_eq!(
        token_balance(&svm, &accounts.vault_x),
        DEPOSIT_X + DEPOSIT_X / 2
    );
    assert_eq!(
        token_balance(&svm, &accounts.vault_y),
        DEPOSIT_Y + DEPOSIT_Y / 2
    );
    assert_eq!(
        token_balance(&svm, &accounts.user_x),
        USER_MINT_FUNDING - DEPOSIT_X - DEPOSIT_X / 2
    );
    assert_eq!(
        token_balance(&svm, &accounts.user_y),
        USER_MINT_FUNDING - DEPOSIT_Y - DEPOSIT_Y / 2
    );
    assert_eq!(
        token_balance(&svm, &accounts.user_lp),
        LP_MINT + LP_MINT / 2
    );
}

#[test]
fn test_deposit_zero_max_on_first_deposit_fails() {
    let PoolFixture { mut svm, accounts } = setup();

    let init_ix = create_initialize_tx(&accounts, 30);
    send(&mut svm, &[&accounts.payer], init_ix).expect("initialize should succeed");

    // Zero max on either side mints nothing: no free LP on an empty pool.
    for (max_x, max_y) in [(0, DEPOSIT_Y), (DEPOSIT_X, 0)] {
        let deposit_ix = create_deposit_tx(&accounts, LP_MINT, max_x, max_y);
        let err = send(&mut svm, &[&accounts.payer], deposit_ix).unwrap_err();

        let expected = AmmError::InvalidAmount.to_string();
        let err_str = format!("{err:?}");
        assert!(
            err_str.contains(&expected),
            "expected {expected}, got: {err_str}"
        );
    }
}

#[test]
fn test_deposit_minimal_amount_fails() {
    let PoolFixture { mut svm, accounts } = setup_initialized_pool();

    // Minting LP for nothing is rejected instead of rounding down to a free mint
    let deposit_ix = create_deposit_tx(&accounts, 1, 200, 300);
    let err = send(&mut svm, &[&accounts.payer], deposit_ix).unwrap_err();

    let expected = AmmError::InvalidAmount.to_string();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains(&expected),
        "expected {expected}, got: {err_str}"
    );
}
