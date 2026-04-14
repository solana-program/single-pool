#![allow(dead_code)] // needed because cargo doesn't understand test usage
#![allow(clippy::arithmetic_side_effects)]

use {
    crate::get_account,
    bincode::deserialize,
    solana_account::AccountSharedData,
    solana_hash::Hash,
    solana_keypair::Keypair,
    solana_native_token::LAMPORTS_PER_SOL,
    solana_program_test::BanksClient,
    solana_program_test::ProgramTestContext,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_stake_interface::{
        instruction as stake_instruction, program as stake_program,
        stake_flags::StakeFlags,
        state::{Authorized, Delegation, Lockup, Meta, Stake, StakeStateV2},
    },
    solana_system_interface::instruction as system_instruction,
    solana_transaction::Transaction,
    std::convert::TryInto,
};

pub const TEST_STAKE_AMOUNT: u64 = 10_000_000_000; // 10 sol
pub const MANGLED_DELEGATION: u64 = 12345;

pub async fn get_stake_account(
    banks_client: &mut BanksClient,
    pubkey: &Pubkey,
) -> (Meta, Option<Stake>, u64) {
    let stake_account = get_account(banks_client, pubkey).await;
    let lamports = stake_account.lamports;
    match deserialize::<StakeStateV2>(&stake_account.data).unwrap() {
        StakeStateV2::Initialized(meta) => (meta, None, lamports),
        StakeStateV2::Stake(meta, stake, _) => (meta, Some(stake), lamports),
        _ => unimplemented!(),
    }
}

pub async fn get_stake_account_rent(banks_client: &mut BanksClient) -> u64 {
    let rent = banks_client.get_rent().await.unwrap();
    rent.minimum_balance(std::mem::size_of::<StakeStateV2>())
}

pub async fn get_minimum_delegation(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
) -> u64 {
    let transaction = Transaction::new_signed_with_payer(
        &[stake_instruction::get_minimum_delegation()],
        Some(&payer.pubkey()),
        &[payer],
        *recent_blockhash,
    );
    let mut data = banks_client
        .simulate_transaction(transaction)
        .await
        .unwrap()
        .simulation_details
        .unwrap()
        .return_data
        .unwrap()
        .data;
    data.resize(8, 0);
    data.try_into().map(u64::from_le_bytes).unwrap()
}

pub async fn get_minimum_pool_balance(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
) -> u64 {
    let stake_program_minimum = get_minimum_delegation(banks_client, payer, recent_blockhash).await;
    std::cmp::max(stake_program_minimum, LAMPORTS_PER_SOL)
}

#[allow(clippy::too_many_arguments)]
pub async fn create_independent_stake_account(
    banks_client: &mut BanksClient,
    fee_payer: &Keypair,
    rent_payer: &Keypair,
    recent_blockhash: &Hash,
    stake: &Keypair,
    authorized: &Authorized,
    lockup: &Lockup,
    stake_amount: u64,
) -> u64 {
    let lamports = get_stake_account_rent(banks_client).await + stake_amount;
    let transaction = Transaction::new_signed_with_payer(
        &stake_instruction::create_account(
            &rent_payer.pubkey(),
            &stake.pubkey(),
            authorized,
            lockup,
            lamports,
        ),
        Some(&fee_payer.pubkey()),
        &[fee_payer, rent_payer, stake],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    lamports
}

pub async fn create_blank_stake_account(
    banks_client: &mut BanksClient,
    fee_payer: &Keypair,
    rent_payer: &Keypair,
    recent_blockhash: &Hash,
    stake: &Keypair,
) -> u64 {
    let lamports = get_stake_account_rent(banks_client).await;
    let transaction = Transaction::new_signed_with_payer(
        &[system_instruction::create_account(
            &rent_payer.pubkey(),
            &stake.pubkey(),
            lamports,
            std::mem::size_of::<StakeStateV2>() as u64,
            &stake_program::id(),
        )],
        Some(&fee_payer.pubkey()),
        &[fee_payer, rent_payer, stake],
        *recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    lamports
}

pub async fn delegate_stake_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    stake: &Pubkey,
    authorized: &Keypair,
    vote: &Pubkey,
) {
    let mut transaction = Transaction::new_with_payer(
        &[stake_instruction::delegate_stake(
            stake,
            &authorized.pubkey(),
            vote,
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, authorized], *recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

pub async fn force_deactivate_stake_account(context: &mut ProgramTestContext, pubkey: &Pubkey) {
    let (meta, stake, _) = get_stake_account(&mut context.banks_client, pubkey).await;
    let delegation = Delegation {
        activation_epoch: 0,
        deactivation_epoch: 0,
        // break anything which erroneously uses this in calculations without redelegating
        stake: MANGLED_DELEGATION,
        ..stake.unwrap().delegation
    };
    let mut account_data = vec![0; std::mem::size_of::<StakeStateV2>()];
    bincode::serialize_into(
        &mut account_data[..],
        &StakeStateV2::Stake(
            meta,
            Stake {
                delegation,
                ..stake.unwrap()
            },
            StakeFlags::empty(),
        ),
    )
    .unwrap();

    let mut stake_account = get_account(&mut context.banks_client, pubkey).await;
    stake_account.data = account_data;
    context.set_account(pubkey, &AccountSharedData::from(stake_account));
}
