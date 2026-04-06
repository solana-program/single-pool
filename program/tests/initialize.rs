#![allow(clippy::arithmetic_side_effects)]

mod helpers;

use {
    helpers::*,
    solana_native_token::LAMPORTS_PER_SOL,
    solana_program_pack::Pack,
    solana_program_test::*,
    solana_signer::Signer,
    solana_stake_interface::program as stake_program,
    solana_transaction::Transaction,
    spl_single_pool::{error::SinglePoolError, id, instruction},
    spl_token_interface::state::Mint,
    test_case::test_matrix,
};

#[test_matrix(
    [StakeProgramVersion::Stable, StakeProgramVersion::Beta, StakeProgramVersion::Edge]
)]
#[tokio::test]
async fn minimum_pool_balance(stake_version: StakeProgramVersion) {
    let Some(program_test) = program_test(stake_version) else {
        return;
    };
    let mut context = program_test.start_with_context().await;

    let accounts = SinglePoolAccounts::default();
    let minimum_pool_balance = accounts.initialize(&mut context).await;

    // this test is a canary intended to fail if stake program minimum delegation ever increases.
    // we "back" the 1b lamport minimum pool balance with LAMPORTS_PER_SOL notional pool tokens.
    // we *must not* use `minimum_pool_balance()` for tokens because it could adversely change
    // the economics of pre-existing pools. allowing new pools with >1b starting delegation
    // to have 1b notional tokens is unpleasant, but less frightening, because at least one token
    // will have always meant the same thing for the life of such a pool
    //
    // we arent planning on raising minimum delegation again but this test is a messenger
    // from the past altering you that, if such a thing happens, we may want to do Something
    assert_eq!(
        minimum_pool_balance, LAMPORTS_PER_SOL,
        "Stake Program minimum delegation has changed, token accounting may need to change \
         with it. Please consult the comment for more details.",
    );
}

#[test_matrix(
    [StakeProgramVersion::Stable, StakeProgramVersion::Beta, StakeProgramVersion::Edge]
)]
#[tokio::test]
async fn success(stake_version: StakeProgramVersion) {
    let Some(program_test) = program_test(stake_version) else {
        return;
    };
    let mut context = program_test.start_with_context().await;

    let accounts = SinglePoolAccounts::default();
    accounts.initialize(&mut context).await;

    // mint exists
    let mint_account = get_account(&mut context.banks_client, &accounts.mint).await;
    Mint::unpack_from_slice(&mint_account.data).unwrap();

    // stake account exists
    let stake_account = get_account(&mut context.banks_client, &accounts.stake_account).await;
    assert_eq!(stake_account.owner, stake_program::id());
}

#[tokio::test]
async fn fail_double_init() {
    let mut context = program_test_live().start_with_context().await;
    let accounts = SinglePoolAccounts::default();
    let minimum_pool_balance = accounts.initialize(&mut context).await;
    refresh_blockhash(&mut context).await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let instructions = instruction::initialize(
        &id(),
        &accounts.vote_account.pubkey(),
        &context.payer.pubkey(),
        &rent,
        minimum_pool_balance,
    );
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    let e = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err();
    check_error(e, SinglePoolError::PoolAlreadyInitialized);
}

#[test_matrix(
    [StakeProgramVersion::Stable, StakeProgramVersion::Beta, StakeProgramVersion::Edge]
)]
#[tokio::test]
async fn fail_below_pool_minimum(stake_version: StakeProgramVersion) {
    let Some(program_test) = program_test(stake_version) else {
        return;
    };
    let mut context = program_test.start_with_context().await;

    let accounts = SinglePoolAccounts::default();
    let slot = context.genesis_config().epoch_schedule.first_normal_slot + 1;
    context.warp_to_slot(slot).unwrap();

    create_vote(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &accounts.validator,
        &accounts.voter.pubkey(),
        &accounts.withdrawer.pubkey(),
        &accounts.vote_account,
    )
    .await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let minimum_pool_balance = get_minimum_pool_balance(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
    )
    .await;

    let instructions = instruction::initialize(
        &id(),
        &accounts.vote_account.pubkey(),
        &context.payer.pubkey(),
        &rent,
        minimum_pool_balance - 1,
    );
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    let e = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err();
    check_error(e, SinglePoolError::WrongRentAmount);
}

// TODO test that init can succeed without mpl program
