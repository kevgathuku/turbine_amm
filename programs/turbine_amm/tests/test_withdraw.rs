mod common;

use common::*;

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
