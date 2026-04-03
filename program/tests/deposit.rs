#![allow(clippy::arithmetic_side_effects)]

mod helpers;

use {
    helpers::*,
    solana_clock::Clock,
    solana_program_test::*,
    solana_sdk::{signature::Signer, signer::keypair::Keypair, transaction::Transaction},
    solana_stake_interface::{
        instruction as stake_instruction,
        stake_history::StakeHistory,
        state::{Authorized, Lockup, StakeActivationStatus, StakeStateV2},
    },
    solana_system_interface::instruction as system_instruction,
    spl_associated_token_account_interface::address::get_associated_token_address,
    spl_single_pool::{error::SinglePoolError, id, instruction},
    test_case::test_matrix,
};

#[allow(deprecated)]
use spl_single_pool::find_default_deposit_account_address;

#[test_matrix(
    [StakeProgramVersion::Stable, StakeProgramVersion::Beta, StakeProgramVersion::Edge],
    [false, true],
    [0, 100_000],
    [0, 100_000],
    [false, true],
    [false, true]
)]
#[tokio::test]
async fn success(
    stake_version: StakeProgramVersion,
    activate: bool,
    pool_extra_lamports: u64,
    alice_extra_lamports: u64,
    prior_deposit: bool,
    small_deposit: bool,
) {
    let Some(program_test) = program_test(stake_version) else {
        return;
    };
    let mut context = program_test.start_with_context().await;

    let rent = context.banks_client.get_rent().await.unwrap();
    let rent_exempt_reserve = rent.minimum_balance(StakeStateV2::size_of());

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

    let (_, alice_stake_before_deposit, _) =
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

    let (_, pool_stake_after, pool_lamports_after) =
        get_stake_account(&mut context.banks_client, &accounts.stake_account).await;
    let pool_stake_after = pool_stake_after.unwrap().delegation.stake;

    // when active, the depositor gets their rent and extra back
    // but when activating, rent is added to stake
    let expected_deposit = if activate {
        alice_stake_before_deposit
    } else {
        alice_stake_before_deposit + rent_exempt_reserve
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
        pool_stake_before + expected_deposit + rent_exempt_reserve + pool_extra_lamports,
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

#[test_matrix(
    [StakeProgramVersion::Stable, StakeProgramVersion::Beta, StakeProgramVersion::Edge],
    [false, true],
    [false, true]
)]
#[tokio::test]
async fn success_with_seed(
    stake_version: StakeProgramVersion,
    activate: bool,
    small_deposit: bool,
) {
    let Some(program_test) = program_test(stake_version) else {
        return;
    };
    let mut context = program_test.start_with_context().await;

    let accounts = SinglePoolAccounts::default();
    let rent = context.banks_client.get_rent().await.unwrap();
    let minimum_stake = accounts.initialize(&mut context).await;
    #[allow(deprecated)]
    let alice_default_stake =
        find_default_deposit_account_address(&accounts.pool, &accounts.alice.pubkey());

    #[allow(deprecated)]
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

#[test_matrix(
    [StakeProgramVersion::Stable, StakeProgramVersion::Beta, StakeProgramVersion::Edge],
    [false, true]
)]
#[tokio::test]
async fn fail_uninitialized(stake_version: StakeProgramVersion, activate: bool) {
    let Some(program_test) = program_test(stake_version) else {
        return;
    };
    let mut context = program_test.start_with_context().await;

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

    let token_account = get_associated_token_address(&context.payer.pubkey(), &accounts.mint);

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

#[test_matrix(
    [StakeProgramVersion::Stable, StakeProgramVersion::Beta, StakeProgramVersion::Edge],
    [false, true],
    [BadDeposit::User, BadDeposit::Pool, BadDeposit::Onramp]
)]
#[tokio::test]
async fn fail_bad_account(
    stake_version: StakeProgramVersion,
    activate: bool,
    deposit_source: BadDeposit,
) {
    let Some(program_test) = program_test(stake_version) else {
        return;
    };
    let mut context = program_test.start_with_context().await;

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
        check_error(e, SinglePoolError::WrongStakeState);
    } else {
        check_error(e, SinglePoolError::InvalidPoolStakeAccountUsage);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UserStakeState {
    Initialized,
    Activating,
    Active,
    Deactivating,
    Deactive,
}

#[test_matrix(
    [StakeProgramVersion::Stable, StakeProgramVersion::Beta, StakeProgramVersion::Edge],
    [false, true],
    [UserStakeState::Initialized, UserStakeState::Activating, UserStakeState::Active,
     UserStakeState::Deactivating, UserStakeState::Deactive]
)]
#[tokio::test]
async fn all_activation_states(
    stake_version: StakeProgramVersion,
    activate: bool,
    user_stake_state: UserStakeState,
) {
    let Some(program_test) = program_test(stake_version) else {
        return;
    };
    let mut context = program_test.start_with_context().await;

    let accounts = SinglePoolAccounts::default();

    let stake_amount = get_minimum_pool_balance(
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

    // warp immediately so our user stake does not have an activation epoch of 0
    let second_normal_slot = context.genesis_config().epoch_schedule.first_normal_slot + 1;
    context.warp_to_slot(second_normal_slot).unwrap();

    // if user stake is active or deactivating, create it immediately
    if user_stake_state == UserStakeState::Active
        || user_stake_state == UserStakeState::Deactivating
    {
        create_independent_stake_account(
            &mut context.banks_client,
            &context.payer,
            &context.payer,
            &context.last_blockhash,
            &accounts.alice_stake,
            &Authorized::auto(&accounts.alice.pubkey()),
            &Lockup::default(),
            stake_amount,
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

        advance_epoch(&mut context).await;
    }

    // create and delegate the pool
    // we do not need to test a deactivated pool, replenish tests cover this
    accounts.initialize(&mut context).await;

    // if pool should be active, advance
    if activate {
        advance_epoch(&mut context).await;
    }

    // now if user stake is initialized, activating, or deactive, create it
    // we now have a user stake for all test cases
    if user_stake_state == UserStakeState::Activating
        || user_stake_state == UserStakeState::Deactive
    {
        create_independent_stake_account(
            &mut context.banks_client,
            &context.payer,
            &context.payer,
            &context.last_blockhash,
            &accounts.alice_stake,
            &Authorized::auto(&accounts.alice.pubkey()),
            &Lockup::default(),
            stake_amount,
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
    } else if user_stake_state == UserStakeState::Initialized {
        let lamports = get_stake_account_rent(&mut context.banks_client).await + stake_amount;

        let transaction = Transaction::new_signed_with_payer(
            &stake_instruction::create_account(
                &context.payer.pubkey(),
                &accounts.alice_stake.pubkey(),
                &Authorized::auto(&accounts.alice.pubkey()),
                &Lockup::default(),
                lamports,
            ),
            Some(&context.payer.pubkey()),
            &[&context.payer, &accounts.alice_stake],
            context.last_blockhash,
        );

        context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();
    }

    // this creates the deactivating account (it was active) *and* the deactive account (it was activating)
    if user_stake_state == UserStakeState::Deactivating
        || user_stake_state == UserStakeState::Deactive
    {
        let transaction = Transaction::new_signed_with_payer(
            &[stake_instruction::deactivate_stake(
                &accounts.alice_stake.pubkey(),
                &accounts.alice.pubkey(),
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

    // the above is an unpleasant machine. sanity check both accounts
    let activating = StakeActivationStatus::with_effective_and_activating(0, stake_amount);
    let active = StakeActivationStatus::with_effective(stake_amount);
    let deactivating = StakeActivationStatus::with_deactivating(stake_amount);
    let deactive = StakeActivationStatus::default();

    let clock = context.banks_client.get_sysvar::<Clock>().await.unwrap();
    let stake_history = context
        .banks_client
        .get_sysvar::<StakeHistory>()
        .await
        .unwrap();

    let pool_triple = get_stake_account(&mut context.banks_client, &accounts.stake_account)
        .await
        .1
        .unwrap()
        .delegation
        .stake_activating_and_deactivating(clock.epoch, &stake_history, Some(0));

    if activate {
        assert_eq!(pool_triple, active);
    } else {
        assert_eq!(pool_triple, activating);
    }

    let user_triple = get_stake_account(&mut context.banks_client, &accounts.alice_stake.pubkey())
        .await
        .1
        .map(|stake| {
            stake
                .delegation
                .stake_activating_and_deactivating(clock.epoch, &stake_history, Some(0))
        });

    match user_stake_state {
        UserStakeState::Initialized => assert!(user_triple.is_none()),
        UserStakeState::Activating => assert_eq!(user_triple.unwrap(), activating),
        UserStakeState::Active => assert_eq!(user_triple.unwrap(), active),
        UserStakeState::Deactivating => assert_eq!(user_triple.unwrap(), deactivating),
        UserStakeState::Deactive => assert_eq!(user_triple.unwrap(), deactive),
    }

    // finally we can run the deposit
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

    if (activate && user_stake_state == UserStakeState::Active)
        || (!activate
            && [
                UserStakeState::Initialized,
                UserStakeState::Activating,
                UserStakeState::Deactive,
            ]
            .contains(&user_stake_state))
    {
        context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();
    } else {
        let e = context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err();
        check_error(e, SinglePoolError::WrongStakeState);
    }
}
