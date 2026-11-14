mod common;

use {
    common::*,
};


#[test]
fn test_initialize() {
    let PoolFixture { mut svm, accounts } = setup();

    let instruction = create_initialize_tx(
        &mut svm,
        &accounts.payer,
        accounts.mint_x,
        accounts.mint_y,
        accounts.pool,
        accounts.mint_lp,
        accounts.vault_x,
        accounts.vault_y,
    );
    let res = send(
        &mut svm,
        &[instruction],
        &accounts.payer,
        &[&accounts.payer],
    );
    assert!(res.is_ok());
}
