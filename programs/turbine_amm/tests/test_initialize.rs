mod common;

use {common::*, turbine_amm::error::AmmError};

#[test]
fn test_initialize() {
    let PoolFixture { mut svm, accounts } = setup();

    let instruction = create_initialize_tx(&mut svm, &accounts, 30);
    let res = send(
        &mut svm,
        &[instruction],
        &accounts.payer,
        &[&accounts.payer],
    );
    assert!(res.is_ok());
}

#[test]
fn test_initialize_excessive_fee_fails() {
    use turbine_amm::constants::MAX_FEE_BPS;

    let PoolFixture { mut svm, accounts } = setup();

    let instruction = create_initialize_tx(&mut svm, &accounts, MAX_FEE_BPS + 1);
    let err = send(
        &mut svm,
        &[instruction],
        &accounts.payer,
        &[&accounts.payer],
    )
    .unwrap_err();

    let expected = AmmError::FeePercentErr.to_string();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains(&expected),
        "expected FeePercentErr, got: {err_str}"
    );
}
