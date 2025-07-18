#![allow(clippy::arithmetic_side_effects)]
#![cfg(feature = "test-sbf")]

mod helpers;

use {
    helpers::*,
    solana_program_test::*,
    solana_sdk::{signature::Signer, signer::keypair::Keypair, transaction::Transaction},
    solana_stake_interface::state::{Authorized, Lockup},
    solana_system_interface::instruction as system_instruction,
    spl_associated_token_account_client::address as atoken,
    spl_single_pool::{
        error::SinglePoolError, find_default_deposit_account_address, id, instruction,
    },
    test_case::test_case,
};

#[test_case(true, 0, 0, false, false, false; "activated::minimum_disabled")]
#[test_case(true, 0, 0, false, false, true; "activated::minimum_disabled::small")]
#[test_case(true, 0, 0, false, true, false; "activated::minimum_enabled")]
#[test_case(false, 0, 0, false, false, false; "activating::minimum_disabled")]
#[test_case(false, 0, 0, false, false, true; "activating::minimum_disabled::small")]
#[test_case(false, 0, 0, false, true, false; "activating::minimum_enabled")]
#[test_case(true, 100_000, 0, false, false, false; "activated::extra_none")]
#[test_case(false, 100_000, 0, false, false, false; "activating::extra_none")]
#[test_case(true, 0, 100_000, false, false, false; "activated::none_extra")]
#[test_case(false, 0, 100_000, false, false, false; "activating::none_extra")]
#[test_case(true, 100_000, 100_000, false, false, false; "activated::extra_extra")]
#[test_case(false, 100_000, 100_000, false, false, false; "activating::extra_extra")]
#[test_case(true, 0, 0, true, false, false; "activated::second")]
#[test_case(false, 0, 0, true, false, false; "activating::second")]
#[tokio::test]
async fn success(
    activate: bool,
    pool_extra_lamports: u64,
    alice_extra_lamports: u64,
    prior_deposit: bool,
    enable_minimum_delegation: bool,
    small_deposit: bool,
) {
    let mut context = program_test(enable_minimum_delegation)
        .start_with_context()
        .await;
    let accounts = SinglePoolAccounts::default();
    accounts
        .initialize_for_deposit(
            &mut context,
            if small_deposit { 1 } else { TEST_STAKE_AMOUNT },
            if prior_deposit {
                Some(TEST_STAKE_AMOUNT * 10)
            } else {
                None
            },
        )
        .await;

    if activate {
        advance_epoch(&mut context).await;
    }

    if prior_deposit {
        let instructions = instruction::deposit(
            &id(),
            &accounts.pool,
            &accounts.bob_stake.pubkey(),
            &accounts.bob_token,
            &accounts.bob.pubkey(),
            &accounts.bob.pubkey(),
        );
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&context.payer.pubkey()),
            &[&context.payer, &accounts.bob],
            context.last_blockhash,
        );

        context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();
    }

    if pool_extra_lamports > 0 {
        transfer(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &accounts.stake_account,
            pool_extra_lamports,
        )
        .await;
    }

    if alice_extra_lamports > 0 {
        let transaction = Transaction::new_signed_with_payer(
            &[system_instruction::transfer(
                &accounts.alice.pubkey(),
                &accounts.alice_stake.pubkey(),
                alice_extra_lamports,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, &accounts.alice],
            context.last_blockhash,
        );
        context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();
    }

    let (alice_meta_before_deposit, alice_stake_before_deposit, _) =
        get_stake_account(&mut context.banks_client, &accounts.alice_stake.pubkey()).await;
    let alice_stake_before_deposit = alice_stake_before_deposit.unwrap().delegation.stake;

    let (_, pool_stake_before, pool_lamports_before) =
        get_stake_account(&mut context.banks_client, &accounts.stake_account).await;
    let pool_stake_before = pool_stake_before.unwrap().delegation.stake;

    let instructions = instruction::deposit(
        &id(),
        &accounts.pool,
        &accounts.alice_stake.pubkey(),
        &accounts.alice_token,
        &accounts.alice.pubkey(),
        &accounts.alice.pubkey(),
    );
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer, &accounts.alice],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let wallet_lamports_after_deposit =
        get_account(&mut context.banks_client, &accounts.alice.pubkey())
            .await
            .lamports;

    let (pool_meta_after, pool_stake_after, pool_lamports_after) =
        get_stake_account(&mut context.banks_client, &accounts.stake_account).await;
    let pool_stake_after = pool_stake_after.unwrap().delegation.stake;

    // when active, the depositor gets their rent and extra back
    // but when activating, rent is added to stake
    let expected_deposit = if activate {
        alice_stake_before_deposit
    } else {
        alice_stake_before_deposit + alice_meta_before_deposit.rent_exempt_reserve
    };

    // deposit stake account is closed
    assert!(context
        .banks_client
        .get_account(accounts.alice_stake.pubkey())
        .await
        .expect("get_account")
        .is_none());

    // entire stake has moved to pool
    assert_eq!(pool_stake_before + expected_deposit, pool_stake_after);

    // pool only gained stake, pool kept any extra lamports it had
    assert_eq!(pool_lamports_after, pool_lamports_before + expected_deposit);
    assert_eq!(
        pool_lamports_after,
        pool_stake_before
            + expected_deposit
            + pool_meta_after.rent_exempt_reserve
            + pool_extra_lamports,
    );

    // alice got her rent and extra back if active, or just extra back otherwise
    assert_eq!(
        wallet_lamports_after_deposit,
        USER_STARTING_LAMPORTS - expected_deposit,
    );

    // alice got tokens. no rewards have been paid so tokens correspond to stake 1:1
    assert_eq!(
        get_token_balance(&mut context.banks_client, &accounts.alice_token).await,
        expected_deposit,
    );
}

#[test_case(true, false, false; "activated::minimum_disabled")]
#[test_case(true, false, true; "activated::minimum_disabled::small")]
#[test_case(true, true, false; "activated::minimum_enabled")]
#[test_case(false, false, false; "activating::minimum_disabled")]
#[test_case(false, false, true; "activating::minimum_disabled::small")]
#[test_case(false, true, false; "activating::minimum_enabled")]
#[tokio::test]
async fn success_with_seed(activate: bool, enable_minimum_delegation: bool, small_deposit: bool) {
    let mut context = program_test(enable_minimum_delegation)
        .start_with_context()
        .await;
    let accounts = SinglePoolAccounts::default();
    let rent = context.banks_client.get_rent().await.unwrap();
    let minimum_stake = accounts.initialize(&mut context).await;
    let alice_default_stake =
        find_default_deposit_account_address(&accounts.pool, &accounts.alice.pubkey());

    let instructions = instruction::create_and_delegate_user_stake(
        &id(),
        &accounts.vote_account.pubkey(),
        &accounts.alice.pubkey(),
        &rent,
        if small_deposit { 1 } else { minimum_stake },
    );
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer, &accounts.alice],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    if activate {
        advance_epoch(&mut context).await;
    }

    let (_, alice_stake_before_deposit, stake_lamports) =
        get_stake_account(&mut context.banks_client, &alice_default_stake).await;
    let alice_stake_before_deposit = alice_stake_before_deposit.unwrap().delegation.stake;

    let instructions = instruction::deposit(
        &id(),
        &accounts.pool,
        &alice_default_stake,
        &accounts.alice_token,
        &accounts.alice.pubkey(),
        &accounts.alice.pubkey(),
    );
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer, &accounts.alice],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let wallet_lamports_after_deposit =
        get_account(&mut context.banks_client, &accounts.alice.pubkey())
            .await
            .lamports;

    let (_, pool_stake_after, _) =
        get_stake_account(&mut context.banks_client, &accounts.stake_account).await;
    let pool_stake_after = pool_stake_after.unwrap().delegation.stake;

    let expected_deposit = if activate {
        alice_stake_before_deposit
    } else {
        stake_lamports
    };

    // deposit stake account is closed
    assert!(context
        .banks_client
        .get_account(alice_default_stake)
        .await
        .expect("get_account")
        .is_none());

    // stake moved to pool
    assert_eq!(minimum_stake + expected_deposit, pool_stake_after);

    // alice got her rent back if active, or everything otherwise
    assert_eq!(
        wallet_lamports_after_deposit,
        USER_STARTING_LAMPORTS - expected_deposit
    );

    // alice got tokens. no rewards have been paid so tokens correspond to stake 1:1
    assert_eq!(
        get_token_balance(&mut context.banks_client, &accounts.alice_token).await,
        expected_deposit,
    );
}

#[test_case(true; "activated")]
#[test_case(false; "activating")]
#[tokio::test]
async fn fail_uninitialized(activate: bool) {
    let mut context = program_test(false).start_with_context().await;
    let accounts = SinglePoolAccounts::default();
    let stake_account = Keypair::new();

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

    let token_account =
        atoken::get_associated_token_address(&context.payer.pubkey(), &accounts.mint);

    create_independent_stake_account(
        &mut context.banks_client,
        &context.payer,
        &context.payer,
        &context.last_blockhash,
        &stake_account,
        &Authorized::auto(&context.payer.pubkey()),
        &Lockup::default(),
        TEST_STAKE_AMOUNT,
    )
    .await;

    delegate_stake_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &stake_account.pubkey(),
        &context.payer,
        &accounts.vote_account.pubkey(),
    )
    .await;

    if activate {
        advance_epoch(&mut context).await;
    }

    let instructions = instruction::deposit(
        &id(),
        &accounts.pool,
        &stake_account.pubkey(),
        &token_account,
        &context.payer.pubkey(),
        &context.payer.pubkey(),
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
    check_error(e, SinglePoolError::InvalidPoolAccount);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BadDeposit {
    User,
    Pool,
    Onramp,
}

#[test_case(true, BadDeposit::User; "activated::unauth")]
#[test_case(false, BadDeposit::User; "activating::unauth")]
#[test_case(true, BadDeposit::Pool; "activated::pool")]
#[test_case(false, BadDeposit::Pool; "activating::pool")]
#[test_case(true, BadDeposit::Onramp; "activated::onramp")]
#[tokio::test]
async fn fail_bad_account(activate: bool, deposit_source: BadDeposit) {
    let mut context = program_test(false).start_with_context().await;
    let accounts = SinglePoolAccounts::default();
    accounts
        .initialize_for_deposit(&mut context, TEST_STAKE_AMOUNT, None)
        .await;

    if activate {
        advance_epoch(&mut context).await;
    }

    if deposit_source == BadDeposit::Onramp {
        let minimum_delegation = get_minimum_delegation(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
        )
        .await;

        transfer(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &accounts.onramp_account,
            minimum_delegation,
        )
        .await;

        let instruction = instruction::replenish_pool(&id(), &accounts.vote_account.pubkey());
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );

        context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        advance_epoch(&mut context).await;
    }

    let deposit_source_address = match deposit_source {
        BadDeposit::User => accounts.alice_stake.pubkey(),
        BadDeposit::Pool => accounts.stake_account,
        BadDeposit::Onramp => accounts.onramp_account,
    };

    let instruction = instruction::deposit_stake(
        &id(),
        &accounts.pool,
        &deposit_source_address,
        &accounts.alice_token,
        &accounts.alice.pubkey(),
    );
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&accounts.alice.pubkey()),
        &[&accounts.alice],
        context.last_blockhash,
    );

    let e = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err();

    if deposit_source == BadDeposit::User {
        check_error(e, SinglePoolError::WrongStakeStake);
    } else {
        check_error(e, SinglePoolError::InvalidPoolStakeAccountUsage);
    }
}

#[test_case(true; "pool_active")]
#[test_case(false; "user_active")]
#[tokio::test]
async fn fail_activation_mismatch(pool_first: bool) {
    let mut context = program_test(false).start_with_context().await;
    let accounts = SinglePoolAccounts::default();

    let minimum_pool_balance = get_minimum_pool_balance(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
    )
    .await;

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

    if pool_first {
        accounts.initialize(&mut context).await;
        advance_epoch(&mut context).await;
    }

    create_independent_stake_account(
        &mut context.banks_client,
        &context.payer,
        &context.payer,
        &context.last_blockhash,
        &accounts.alice_stake,
        &Authorized::auto(&accounts.alice.pubkey()),
        &Lockup::default(),
        minimum_pool_balance,
    )
    .await;

    delegate_stake_account(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &accounts.alice_stake.pubkey(),
        &accounts.alice,
        &accounts.vote_account.pubkey(),
    )
    .await;

    if !pool_first {
        advance_epoch(&mut context).await;
        accounts.initialize(&mut context).await;
    }

    let instructions = instruction::deposit(
        &id(),
        &accounts.pool,
        &accounts.alice_stake.pubkey(),
        &accounts.alice_token,
        &accounts.alice.pubkey(),
        &accounts.alice.pubkey(),
    );
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&accounts.alice.pubkey()),
        &[&accounts.alice],
        context.last_blockhash,
    );

    let e = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err();
    check_error(e, SinglePoolError::WrongStakeStake);
}
