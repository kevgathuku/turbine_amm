mod common;

use {
    anchor_spl::associated_token::get_associated_token_address, common::*,
    turbine_amm::error::AmmError,
};

#[test]
fn test_initialize() {
    let PoolFixture { mut svm, accounts } = setup();

    let ix = create_initialize_tx(&accounts, 30);
    send(&mut svm, &[&accounts.payer], ix).expect("initialize should succeed");

    // Treasury ATAs are pre-created at initialize so swaps never init.
    assert_eq!(
        token_balance(
            &svm,
            &get_associated_token_address(&accounts.treasury, &accounts.mint_x)
        ),
        0
    );
    assert_eq!(
        token_balance(
            &svm,
            &get_associated_token_address(&accounts.treasury, &accounts.mint_y)
        ),
        0
    );
}

#[test]
fn test_initialize_excessive_fee_fails() {
    use turbine_amm::constants::MAX_FEE_BPS;

    let PoolFixture { mut svm, accounts } = setup();

    let ix = create_initialize_tx(&accounts, MAX_FEE_BPS + 1);
    let err = send(&mut svm, &[&accounts.payer], ix).unwrap_err();

    let expected = AmmError::FeePercentErr.to_string();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains(&expected),
        "expected {expected}, got: {err_str}"
    );
}

#[test]
fn test_initialize_mismatched_decimals_fails() {
    let PoolFixture { mut svm, accounts } = setup();

    let mint_9 = create_mint_with_decimals(&mut svm, &accounts.payer, 9);
    let bad_accounts = PoolAccounts {
        mint_y: mint_9,
        vault_y: get_associated_token_address(&accounts.pool, &mint_9),
        ..accounts
    };
    let ix = create_initialize_tx(&bad_accounts, 30);
    let err = send(&mut svm, &[&bad_accounts.payer], ix).unwrap_err();

    let expected = AmmError::MismatchedDecimals.to_string();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains(&expected),
        "expected {expected}, got: {err_str}"
    );
}
