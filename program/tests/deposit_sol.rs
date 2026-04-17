#![allow(clippy::arithmetic_side_effects)]

mod helpers;

use {
    helpers::*,
    solana_account::AccountSharedData,
    solana_keypair::Keypair,
    solana_native_token::LAMPORTS_PER_SOL,
    solana_program_test::*,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_transaction::Transaction,
    spl_single_pool::{error::SinglePoolError, id, instruction},
    test_case::test_matrix,
};

async fn deposit_sol(
    context: &mut ProgramTestContext,
    accounts: &SinglePoolAccounts,
    lamports: u64,
) -> Result<(), BanksClientError> {
    let proxy_keypair = Keypair::new();

    let instructions = instruction::deposit_liquid(
        &id(),
        &accounts.vote_account.pubkey(),
        &accounts.alice.pubkey(),
        &proxy_keypair.pubkey(),
        &accounts.alice_token,
        lamports,
    );
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer, &accounts.alice, &proxy_keypair],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(transaction).await
}

#[test_matrix(
    [StakeProgramVersion::Stable, StakeProgramVersion::Beta, StakeProgramVersion::Edge],
    [1000, TEST_STAKE_AMOUNT],
    [0, LAMPORTS_PER_SOL * 3]
)]
#[tokio::test]
async fn success(
    stake_version: StakeProgramVersion,
    deposit_amount: u64,
    additional_pool_value: u64,
) {
    let Some(program_test) = program_test(stake_version) else {
        return;
    };
    let mut context = program_test.start_with_context().await;

    let accounts = SinglePoolAccounts::default();
    let minimum_pool_balance = accounts.initialize(&mut context).await;

    advance_epoch(&mut context).await;

    let minimum_delegation = get_minimum_delegation(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
    )
    .await;

    if additional_pool_value > 0 {
        transfer(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &accounts.onramp_account,
            additional_pool_value,
        )
        .await;
    }

    let wallet_lamports_before = get_account(&mut context.banks_client, &accounts.alice.pubkey())
        .await
        .lamports;

    let onramp_lamports_before = get_account(&mut context.banks_client, &accounts.onramp_account)
        .await
        .lamports;

    // phantom tokens
    let token_supply_before =
        get_token_supply(&mut context.banks_client, &accounts.mint).await + LAMPORTS_PER_SOL;

    assert_eq!(
        0,
        get_token_balance(&mut context.banks_client, &accounts.alice_token).await
    );

    // note our 1000 deposit case also tests that we *dont* need our proxy to make rent
    deposit_sol(&mut context, &accounts, deposit_amount)
        .await
        .unwrap();

    let wallet_lamports_after = get_account(&mut context.banks_client, &accounts.alice.pubkey())
        .await
        .lamports;

    // we deposited the expected amount
    assert_eq!(
        deposit_amount,
        wallet_lamports_before - wallet_lamports_after,
    );

    let (_, onramp_stake_after, onramp_lamports_after) =
        get_stake_account(&mut context.banks_client, &accounts.onramp_account).await;

    // pool duly recieved said deposit
    assert_eq!(
        deposit_amount,
        onramp_lamports_after - onramp_lamports_before,
    );

    // onramp was successfully replenished if we met the minimum delegation
    if (deposit_amount + additional_pool_value) >= minimum_delegation {
        assert_eq!(
            deposit_amount + additional_pool_value,
            onramp_stake_after.unwrap().delegation.stake
        );
    }

    let user_tokens_after =
        get_token_balance(&mut context.banks_client, &accounts.alice_token).await;

    // depositing n stake yields n tokens for an initial pool with 1b locked stake to 1b phantom tokens
    // so if we deposit n lamports we expect to cleanly have 1% fewer tokens than that
    // if the pool has more stake per token we have to scale our expectations by token supply
    // branch here so the simple case gets tested wth simple math
    if additional_pool_value == 0 {
        assert_eq!(deposit_amount - deposit_amount / 100, user_tokens_after);
    } else {
        let raw_tokens =
            deposit_amount * token_supply_before / (minimum_pool_balance + additional_pool_value);

        assert_eq!(raw_tokens - raw_tokens / 100, user_tokens_after);
    }
}

#[test_matrix(
    [StakeProgramVersion::Stable, StakeProgramVersion::Beta, StakeProgramVersion::Edge]
)]
#[tokio::test]
async fn fail_bad_pool(stake_version: StakeProgramVersion) {
    let Some(program_test) = program_test(stake_version) else {
        return;
    };
    let mut context = program_test.start_with_context().await;

    let accounts = SinglePoolAccounts::default();
    accounts.initialize(&mut context).await;

    let e = deposit_sol(&mut context, &accounts, TEST_STAKE_AMOUNT)
        .await
        .unwrap_err();

    // fail: pool activating
    check_error(e, SinglePoolError::WrongStakeState);

    force_deactivate_stake_account(&mut context, &accounts.stake_account).await;
    refresh_blockhash(&mut context).await;

    let e = deposit_sol(&mut context, &accounts, TEST_STAKE_AMOUNT)
        .await
        .unwrap_err();

    // fail: pool inactive
    check_error(e, SinglePoolError::WrongStakeState);

    replenish(&mut context, &accounts.vote_account.pubkey()).await;
    advance_epoch(&mut context).await;
    context.set_account(&accounts.onramp_account, &AccountSharedData::default());

    let e = deposit_sol(&mut context, &accounts, TEST_STAKE_AMOUNT)
        .await
        .unwrap_err();

    // fail: no onramp
    check_error(e, SinglePoolError::OnRampDoesntExist);
}

#[test_matrix(
    [StakeProgramVersion::Stable, StakeProgramVersion::Beta, StakeProgramVersion::Edge]
)]
#[tokio::test]
async fn fail_bad_deposit(stake_version: StakeProgramVersion) {
    let Some(program_test) = program_test(stake_version) else {
        return;
    };
    let mut context = program_test.start_with_context().await;

    let accounts = SinglePoolAccounts::default();
    accounts.initialize(&mut context).await;

    advance_epoch(&mut context).await;

    let e = deposit_sol(&mut context, &accounts, 0).await.unwrap_err();

    // fail: zero deposit
    check_error(e, SinglePoolError::DepositTooSmall);

    let e = deposit_sol(&mut context, &accounts, 99).await.unwrap_err();

    // fail: deposit rounds to no fee
    check_error(e, SinglePoolError::DepositTooSmall);

    let proxy_keypair = Keypair::new();
    transfer(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &proxy_keypair.pubkey(),
        TEST_STAKE_AMOUNT,
    )
    .await;

    let instruction = instruction::deposit_sol(
        &id(),
        &accounts.vote_account.pubkey(),
        &proxy_keypair.pubkey(),
        &accounts.alice_token,
        TEST_STAKE_AMOUNT + 1,
    );
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &[&context.payer, &proxy_keypair],
        context.last_blockhash,
    );

    let e = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err();

    // fail: not enough for deposit
    check_error(e, SinglePoolError::InvalidDepositSolSource);

    let mut instruction = instruction::deposit_sol(
        &id(),
        &accounts.vote_account.pubkey(),
        &proxy_keypair.pubkey(),
        &accounts.alice_token,
        TEST_STAKE_AMOUNT,
    );
    instruction.accounts[7].is_signer = false;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    let e = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err();

    // fail: missing signer
    check_error(e, SinglePoolError::InvalidDepositSolSource);

    let instruction = instruction::deposit_sol(
        &id(),
        &accounts.vote_account.pubkey(),
        &proxy_keypair.pubkey(),
        &accounts.alice_token,
        TEST_STAKE_AMOUNT,
    );
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.payer.pubkey()),
        &[&context.payer, &proxy_keypair],
        context.last_blockhash,
    );

    context.set_account(
        &proxy_keypair.pubkey(),
        &AccountSharedData::new(TEST_STAKE_AMOUNT, 0, &Pubkey::new_unique()),
    );

    let e = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err();

    // fail: bad owner
    check_error(e, SinglePoolError::InvalidDepositSolSource);
}
