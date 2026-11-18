mod common;

use {common::*, solana_keypair::Keypair, solana_signer::Signer, turbine_amm::error::AmmError};

#[test]
fn test_update_lock_unlock_roundtrip() {
    let PoolFixture { mut svm, accounts } = setup_initialized_pool();

    send(
        &mut svm,
        &[&accounts.payer],
        create_update_tx(&accounts, true),
    )
    .expect("lock should succeed");

    send(
        &mut svm,
        &[&accounts.payer],
        create_update_tx(&accounts, false),
    )
    .expect("unlock should succeed");

    let deposit_ix = create_deposit_tx(&accounts, LP_MINT, DEPOSIT_X, DEPOSIT_Y);
    send(&mut svm, &[&accounts.payer], deposit_ix).expect("deposit should succeed after unlock");
}

#[test]
fn test_update_wrong_authority_fails() {
    let PoolFixture { mut svm, accounts } = setup_initialized_pool();

    // Rogue pays its own fee and signs, but is not the recorded authority.
    let rogue = Keypair::new();
    svm.airdrop(&rogue.pubkey(), 2 * LAMPORTS_PER_SOL).unwrap();

    let ix = create_update_tx_with_authority(&accounts, &rogue.pubkey(), true);
    let err = send(&mut svm, &[&rogue], ix).unwrap_err();

    let expected = AmmError::InvalidAuthority.to_string();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains(&expected),
        "expected {expected}, got: {err_str}"
    );

    // The rejected update changed nothing: the real authority can still lock,
    // and the lock takes effect.
    send(
        &mut svm,
        &[&accounts.payer],
        create_update_tx(&accounts, true),
    )
    .expect("lock should succeed");

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
fn test_update_lock_idempotent_succeeds() {
    let PoolFixture { mut svm, accounts } = setup_initialized_pool();

    // Locking twice is a no-op, not an error: retries must stay safe.
    send(&mut svm, &[&accounts.payer], create_update_tx(&accounts, true))
        .expect("first lock should succeed");
    send(&mut svm, &[&accounts.payer], create_update_tx(&accounts, true))
        .expect("second lock should succeed");
}
