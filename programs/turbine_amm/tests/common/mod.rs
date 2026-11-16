#![allow(dead_code)]

use {
    anchor_lang::{
        prelude::Pubkey,
        solana_program::{instruction::Instruction, system_program},
        InstructionData, ToAccountMetas,
    },
    anchor_spl::{
        associated_token::{get_associated_token_address, ID as ATA_PROGRAM_ID},
        token::spl_token::state::Account as TokenAccount,
    },
    litesvm::LiteSVM,
    litesvm_token::{CreateAssociatedTokenAccount, CreateMint, MintTo},
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_program_pack::Pack,
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
    turbine_amm::constants::{MINT_LP_SEED, POOL_SEED},
};

pub use litesvm_token::spl_token::ID as TOKEN_PROGRAM_ID;

pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;
// Decimals for both mint tokens
pub const DECIMALS: u8 = 6;
// Test scenario amounts: X and Y deliberately differ so an x/y swap fails loudly.
pub const USER_MINT_FUNDING: u64 = 1_000_000_000;
// Fee in basis points charged on swaps; Mirrors the pool's configured fee.
pub const FEE_BPS: u16 = 30;
pub const DEPOSIT_X: u64 = 200_000_000;
pub const DEPOSIT_Y: u64 = 300_000_000;
pub const LP_MINT: u64 = 100_000_000;

#[allow(clippy::result_large_err)]
pub fn send(
    svm: &mut LiteSVM,
    signers: &[&Keypair],
    ix: Instruction,
) -> litesvm::types::TransactionResult {
    svm.expire_blockhash();

    let payer = signers[0].pubkey();
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&payer), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), signers).unwrap();

    svm.send_transaction(tx)
}

pub struct PoolAccounts {
    pub payer: Keypair,
    pub treasury: Pubkey,
    pub mint_lp: Pubkey,
    pub mint_bump: u8,
    pub mint_x: Pubkey,
    pub mint_y: Pubkey,
    pub pool: Pubkey,
    pub pool_bump: u8,
    pub vault_x: Pubkey,
    pub vault_y: Pubkey,
    pub user_x: Pubkey,
    pub user_y: Pubkey,
    pub user_lp: Pubkey,
}

pub struct PoolFixture {
    pub svm: LiteSVM,
    pub accounts: PoolAccounts,
}

// New mint with the payer as authority.
pub fn create_mint(svm: &mut LiteSVM, payer: &Keypair) -> Pubkey {
    create_mint_with_decimals(svm, payer, DECIMALS)
}

// New mint with explicit decimals.
pub fn create_mint_with_decimals(svm: &mut LiteSVM, payer: &Keypair, decimals: u8) -> Pubkey {
    CreateMint::new(svm, payer)
        .decimals(decimals)
        .authority(&payer.pubkey())
        .send()
        .unwrap()
}

// New ATA for `owner` plus `amount` freshly minted into it.
pub fn fund_ata(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Pubkey,
    owner: &Pubkey,
    amount: u64,
) -> Pubkey {
    let ata = CreateAssociatedTokenAccount::new(svm, payer, mint)
        .owner(owner)
        .send()
        .unwrap();
    MintTo::new(svm, payer, mint, &ata, amount).send().unwrap();
    ata
}

// Setup function to initialize LiteSVM and create a payer keypair
pub fn setup() -> PoolFixture {
    let program_id = turbine_amm::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/turbine_amm.so"
    ));
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

    let mint_x = create_mint(&mut svm, &payer);
    let mint_y = create_mint(&mut svm, &payer);

    let treasury = Keypair::new();

    let (pool, pool_bump) =
        Pubkey::find_program_address(&[POOL_SEED, &123u64.to_le_bytes()], &program_id);
    let (mint_lp, mint_bump) =
        Pubkey::find_program_address(&[MINT_LP_SEED, pool.as_ref()], &program_id);

    // Derive the PDA for the vault associated token account using the pool PDA and Mint A
    let vault_x = get_associated_token_address(&pool, &mint_x);
    let vault_y = get_associated_token_address(&pool, &mint_y);

    // Fund the user's ATAs; the LP ATA is created on demand by deposit.
    let user = payer.pubkey();
    let user_x = fund_ata(&mut svm, &payer, &mint_x, &user, USER_MINT_FUNDING);
    let user_y = fund_ata(&mut svm, &payer, &mint_y, &user, USER_MINT_FUNDING);
    let user_lp = get_associated_token_address(&user, &mint_lp);

    PoolFixture {
        svm,
        accounts: PoolAccounts {
            payer,
            treasury: treasury.pubkey(),
            mint_lp,
            mint_bump,
            mint_x,
            mint_y,
            pool,
            pool_bump,
            vault_x,
            vault_y,
            user_x,
            user_y,
            user_lp,
        },
    }
}

// Setup plus initialize plus first deposit: the standard prelude for any test
// that acts on a live pool.
pub fn setup_initialized_pool() -> PoolFixture {
    let PoolFixture { mut svm, accounts } = setup();

    let init_ix = create_initialize_tx(&accounts, FEE_BPS);
    send(&mut svm, &[&accounts.payer], init_ix).expect("initialize should succeed");

    let deposit_ix = create_deposit_tx(&accounts, LP_MINT, DEPOSIT_X, DEPOSIT_Y);
    send(&mut svm, &[&accounts.payer], deposit_ix).expect("first deposit should succeed");

    PoolFixture { svm, accounts }
}

// Read a token balance via the SPL Token account layout
pub fn token_balance(svm: &LiteSVM, account: &Pubkey) -> u64 {
    let acct = svm.get_account(account).expect("account not found");
    TokenAccount::unpack(&acct.data).unwrap().amount
}

pub fn create_initialize_tx(accounts: &PoolAccounts, fee: u16) -> Instruction {
    let maker = accounts.payer.pubkey();

    Instruction::new_with_bytes(
        turbine_amm::id(),
        &turbine_amm::instruction::Initialize {
            seed: 123,
            fee,
            authority: Some(maker),
            treasury: accounts.treasury,
        }
        .data(),
        turbine_amm::accounts::Initialize {
            initializer: maker,
            mint_x: accounts.mint_x,
            mint_y: accounts.mint_y,
            mint_lp: accounts.mint_lp,
            vault_x: accounts.vault_x,
            vault_y: accounts.vault_y,
            pool: accounts.pool,
            token_program: TOKEN_PROGRAM_ID,
            associated_token_program: ATA_PROGRAM_ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

pub fn create_deposit_tx(
    accounts: &PoolAccounts,
    amount: u64,
    max_x: u64,
    max_y: u64,
) -> Instruction {
    let user = accounts.payer.pubkey();

    Instruction::new_with_bytes(
        turbine_amm::id(),
        &turbine_amm::instruction::Deposit {
            amount,
            max_x,
            max_y,
        }
        .data(),
        turbine_amm::accounts::Deposit {
            user,
            mint_x: accounts.mint_x,
            mint_y: accounts.mint_y,
            pool: accounts.pool,
            mint_lp: accounts.mint_lp,
            vault_x: accounts.vault_x,
            vault_y: accounts.vault_y,
            user_x: accounts.user_x,
            user_y: accounts.user_y,
            user_lp: accounts.user_lp,
            token_program: TOKEN_PROGRAM_ID,
            associated_token_program: ATA_PROGRAM_ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

pub fn create_update_tx(accounts: &PoolAccounts, locked: bool) -> Instruction {
    create_update_tx_with_authority(accounts, &accounts.payer.pubkey(), locked)
}

pub fn create_update_tx_with_authority(
    accounts: &PoolAccounts,
    authority: &Pubkey,
    locked: bool,
) -> Instruction {
    Instruction::new_with_bytes(
        turbine_amm::id(),
        &turbine_amm::instruction::Update { locked }.data(),
        turbine_amm::accounts::Update {
            authority: *authority,
            pool: accounts.pool,
        }
        .to_account_metas(None),
    )
}

pub fn create_swap_tx(
    accounts: &PoolAccounts,
    is_x: bool,
    amount_in: u64,
    min_amount_out: u64,
) -> Instruction {
    let user = accounts.payer.pubkey();

    Instruction::new_with_bytes(
        turbine_amm::id(),
        &turbine_amm::instruction::Swap {
            is_x,
            amount_in,
            min_amount_out,
        }
        .data(),
        turbine_amm::accounts::Swap {
            user,
            mint_x: accounts.mint_x,
            mint_y: accounts.mint_y,
            pool: accounts.pool,
            mint_lp: accounts.mint_lp,
            vault_x: accounts.vault_x,
            vault_y: accounts.vault_y,
            user_x: get_associated_token_address(&user, &accounts.mint_x),
            user_y: get_associated_token_address(&user, &accounts.mint_y),
            treasury: accounts.treasury,
            treasury_x: get_associated_token_address(&accounts.treasury, &accounts.mint_x),
            treasury_y: get_associated_token_address(&accounts.treasury, &accounts.mint_y),
            token_program: TOKEN_PROGRAM_ID,
            associated_token_program: ATA_PROGRAM_ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

pub fn create_withdraw_tx(
    accounts: &PoolAccounts,
    amount: u64,
    min_x: u64,
    min_y: u64,
) -> Instruction {
    let user = accounts.payer.pubkey();

    Instruction::new_with_bytes(
        turbine_amm::id(),
        &turbine_amm::instruction::Withdraw {
            amount,
            min_x,
            min_y,
        }
        .data(),
        turbine_amm::accounts::Withdraw {
            user,
            mint_x: accounts.mint_x,
            mint_y: accounts.mint_y,
            pool: accounts.pool,
            mint_lp: accounts.mint_lp,
            vault_x: accounts.vault_x,
            vault_y: accounts.vault_y,
            user_x: accounts.user_x,
            user_y: accounts.user_y,
            user_lp: accounts.user_lp,
            token_program: TOKEN_PROGRAM_ID,
            associated_token_program: ATA_PROGRAM_ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}
