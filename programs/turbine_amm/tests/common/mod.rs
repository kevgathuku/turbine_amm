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
    litesvm_token::{
        spl_token::ID as TOKEN_PROGRAM_ID, CreateAssociatedTokenAccount, CreateMint, MintTo,
    },
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_program_pack::Pack,
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
    turbine_amm::constants::{MINT_LP_SEED, POOL_SEED},
};

pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

pub const USER_MINT_FUNDING: u64 = 1_000_000_000;
// Decimals for both mint tokens
pub const DECIMALS: u8 = 6;

#[allow(clippy::result_large_err)]
pub fn send(
    svm: &mut LiteSVM,
    ixs: &[Instruction],
    payer: &Keypair,
    signers: &[&Keypair],
) -> litesvm::types::TransactionResult {
    svm.expire_blockhash();
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(ixs, Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), signers).unwrap();
    svm.send_transaction(tx)
}

pub struct PoolAccounts {
    pub payer: Keypair,
    pub mint_lp: Pubkey,
    pub mint_bump: u8,
    pub mint_x: Pubkey,
    pub mint_y: Pubkey,
    pub pool: Pubkey,
    pub pool_bump: u8,
    pub vault_x: Pubkey,
    pub vault_y: Pubkey,
}

pub struct PoolFixture {
    pub svm: LiteSVM,
    pub accounts: PoolAccounts,
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

    // Create two mints (Mint A and Mint B) with 6 decimal places and the maker as the authority
    let mint_x = CreateMint::new(&mut svm, &payer)
        .decimals(DECIMALS)
        .authority(&payer.pubkey())
        .send()
        .unwrap();

    let mint_y = CreateMint::new(&mut svm, &payer)
        .decimals(DECIMALS)
        .authority(&payer.pubkey())
        .send()
        .unwrap();

    let (pool, pool_bump) =
        Pubkey::find_program_address(&[POOL_SEED, &123u64.to_le_bytes()], &program_id);
    let (mint_lp, mint_bump) =
        Pubkey::find_program_address(&[MINT_LP_SEED, pool.as_ref()], &program_id);

    // Derive the PDA for the vault associated token account using the config PDA and Mint A
    let vault_x = get_associated_token_address(&pool, &mint_x);
    let vault_y = get_associated_token_address(&pool, &mint_y);

    PoolFixture {
        svm,
        accounts: PoolAccounts {
            payer,
            mint_lp,
            mint_bump,
            mint_x,
            mint_y,
            pool,
            pool_bump,
            vault_x,
            vault_y,
        },
    }
}

// Read a token balance via the SPL Token account layout
pub fn token_balance(svm: &LiteSVM, account: &Pubkey) -> u64 {
    let acct = svm.get_account(account).expect("account not found");
    TokenAccount::unpack(&acct.data).unwrap().amount
}

pub fn create_deposit_tx(
    svm: &mut LiteSVM,
    accounts: &PoolAccounts,
    amount: u64,
    max_x: u64,
    max_y: u64,
) -> Instruction {
    let user = accounts.payer.pubkey();

    let user_x = CreateAssociatedTokenAccount::new(svm, &accounts.payer, &accounts.mint_x)
        .owner(&user)
        .send()
        .unwrap();
    MintTo::new(
        svm,
        &accounts.payer,
        &accounts.mint_x,
        &user_x,
        USER_MINT_FUNDING,
    )
    .send()
    .unwrap();

    let user_y = CreateAssociatedTokenAccount::new(svm, &accounts.payer, &accounts.mint_y)
        .owner(&user)
        .send()
        .unwrap();
    MintTo::new(
        svm,
        &accounts.payer,
        &accounts.mint_y,
        &user_y,
        USER_MINT_FUNDING,
    )
    .send()
    .unwrap();

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
            user_x,
            user_y,
            user_lp: get_associated_token_address(&user, &accounts.mint_lp),
            token_program: TOKEN_PROGRAM_ID,
            associated_token_program: ATA_PROGRAM_ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}
pub fn create_initialize_tx(
    mut _svm: &mut LiteSVM,
    accounts: &PoolAccounts,
    fee: u16,
) -> Instruction {
    let maker = accounts.payer.pubkey();

    Instruction::new_with_bytes(
        turbine_amm::id(),
        &turbine_amm::instruction::Initialize {
            seed: 123,
            fee,
            authority: Some(maker),
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
