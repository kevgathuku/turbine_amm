mod common;

use {
    anchor_spl::associated_token::get_associated_token_address, common::*, solana_signer::Signer,
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
