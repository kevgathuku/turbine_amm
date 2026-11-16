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
    let PoolFixture { mut svm, accounts } = setup();

    let init_ix = create_initialize_tx(&accounts, 30);
    send(&mut svm, &[&accounts.payer], init_ix).expect("initialize should succeed");

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
    let PoolFixture { mut svm, accounts } = setup();

    let init_ix = create_initialize_tx(&accounts, 30);
    send(&mut svm, &[&accounts.payer], init_ix).expect("initialize should succeed");

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
