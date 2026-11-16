mod common;

use {
    anchor_lang::error::ErrorCode,
    anchor_spl::associated_token::get_associated_token_address, common::*, solana_signer::Signer,
    turbine_amm::error::AmmError,
};

// X and Y deliberately differ so an x/y swap fails loudly.
const DEPOSIT_X: u64 = 200_000_000;
const DEPOSIT_Y: u64 = 300_000_000;
const LP_MINT: u64 = 100_000_000;

#[test]
fn test_deposit() {
    let PoolFixture { mut svm, accounts } = setup();

    let init_ix = create_initialize_tx(&mut svm, &accounts, 30);
    send(&mut svm, &[init_ix], &accounts.payer, &[&accounts.payer])
        .expect("initialize should succeed");

    let deposit_ix = create_deposit_tx(&mut svm, &accounts, LP_MINT, DEPOSIT_X, DEPOSIT_Y);
    let res = send(&mut svm, &[deposit_ix], &accounts.payer, &[&accounts.payer]);
    assert!(res.is_ok());

    let user = accounts.payer.pubkey();
    let user_x = get_associated_token_address(&user, &accounts.mint_x);
    let user_y = get_associated_token_address(&user, &accounts.mint_y);
    let user_lp = get_associated_token_address(&user, &accounts.mint_lp);
    assert_eq!(token_balance(&svm, &accounts.vault_x), DEPOSIT_X);
    assert_eq!(token_balance(&svm, &accounts.vault_y), DEPOSIT_Y);
    assert_eq!(token_balance(&svm, &user_x), USER_MINT_FUNDING - DEPOSIT_X);
    assert_eq!(token_balance(&svm, &user_y), USER_MINT_FUNDING - DEPOSIT_Y);
    assert_eq!(token_balance(&svm, &user_lp), LP_MINT);
}

#[test]
fn test_deposit_same_mint_fails() {
    let PoolFixture { mut svm, accounts } = setup();

    let init_ix = create_initialize_tx(&mut svm, &accounts, 30);
    send(&mut svm, &[init_ix], &accounts.payer, &[&accounts.payer])
        .expect("initialize should succeed");

    // Same mint for X and Y
    let bad_accounts = PoolAccounts {
        mint_y: accounts.mint_x,
        vault_y: accounts.vault_x,
        ..accounts
    };
    let ix = create_deposit_tx(&mut svm, &bad_accounts, LP_MINT, DEPOSIT_X, DEPOSIT_Y);
    let err = send(&mut svm, &[ix], &bad_accounts.payer, &[&bad_accounts.payer]).unwrap_err();

    // Anchor's duplicate-mutable-account fires before the handler's InvalidToken.
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

    let init_ix = create_initialize_tx(&mut svm, &accounts, 30);
    send(&mut svm, &[init_ix], &accounts.payer, &[&accounts.payer])
        .expect("initialize should succeed");

    // Lock the pool through the real admin instruction.
    let lock_ix = create_update_tx(&accounts, true);
    send(&mut svm, &[lock_ix], &accounts.payer, &[&accounts.payer])
        .expect("lock should succeed");

    let deposit_ix = create_deposit_tx(&mut svm, &accounts, LP_MINT, DEPOSIT_X, DEPOSIT_Y);
    let err = send(
        &mut svm,
        &[deposit_ix],
        &accounts.payer,
        &[&accounts.payer],
    )
    .unwrap_err();

    let logs = err.meta.logs.join("\n");
    let expected = format!("{:?}", AmmError::PoolLocked);
    assert!(logs.contains(&expected), "expected {expected}, got:\n{logs}");
}
