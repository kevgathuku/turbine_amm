mod common;

use {common::*, turbine_amm::error::AmmError};

#[test]
fn test_initialize() {
    let PoolFixture { mut svm, accounts } = setup();

    let ix = create_initialize_tx(&accounts, 30);
    send(&mut svm, &[&accounts.payer], ix).expect("initialize should succeed");
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
