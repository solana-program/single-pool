#![allow(clippy::arithmetic_side_effects)]
#![cfg(feature = "test-sbf")]

mod helpers;

use {
    helpers::*,
    solana_program_test::*,
    solana_sdk::{
        account::AccountSharedData,
        pubkey::Pubkey,
        signature::Signer,
        stake::{
            stake_flags::StakeFlags,
            state::{Delegation, Stake, StakeStateV2},
        },
        sysvar::{clock::Clock, stake_history::StakeHistory},
        transaction::Transaction,
    },
    spl_single_pool::{error::SinglePoolError, id, instruction},
    test_case::test_case,
};

async fn replenish(context: &mut ProgramTestContext, vote_account: &Pubkey) {
    let instruction = instruction::replenish_pool(&id(), vote_account);
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

    refresh_blockhash(context).await;
}

#[test_case(false, false; "noop")]
#[test_case(true, false; "pool")]
#[test_case(false, true; "onramp")]
#[test_case(true, true; "pool_precedence")]
#[tokio::test]
async fn reactivate_success(reactivate_pool: bool, fund_onramp: bool) {
    let mut context = program_test(false).start_with_context().await;
    let accounts = SinglePoolAccounts::default();
    accounts
        .initialize_for_deposit(&mut context, TEST_STAKE_AMOUNT, None)
        .await;
    advance_epoch(&mut context).await;

    // deactivate the pool stake account
    if reactivate_pool {
        let (meta, stake, _) =
            get_stake_account(&mut context.banks_client, &accounts.stake_account).await;
        let delegation = Delegation {
            activation_epoch: 0,
            deactivation_epoch: 0,
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

        let mut stake_account =
            get_account(&mut context.banks_client, &accounts.stake_account).await;
        stake_account.data = account_data;
        context.set_account(
            &accounts.stake_account,
            &AccountSharedData::from(stake_account),
        );

        // make sure deposit fails
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

        let e = context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err();
        check_error(e, SinglePoolError::WrongStakeStake);
    }

    // onramp is already inactive but it doesnt have lamports for delegation
    if fund_onramp {
        let lamports = get_minimum_pool_balance(
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
            lamports,
        )
        .await;
    }

    // replenish and advance
    replenish(&mut context, &accounts.vote_account.pubkey()).await;
    advance_epoch(&mut context).await;

    // deposit works in all cases
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

    assert!(context
        .banks_client
        .get_account(accounts.alice_stake.pubkey())
        .await
        .expect("get_account")
        .is_none());

    // onramp is active now if we transfered to it
    let clock = context.banks_client.get_sysvar::<Clock>().await.unwrap();
    let (_, onramp_stake, _) =
        get_stake_account(&mut context.banks_client, &accounts.onramp_account).await;

    // we require a fully active pool for any onramp state change to reduce complexity
    // the pool for a healthy validator is never unstaked, and a fresh pool you can wait an epoch
    // NOTE we might relax this for DepositSol, in which case this test would change
    if fund_onramp && !reactivate_pool {
        let stake = onramp_stake.unwrap();
        assert_eq!(stake.delegation.activation_epoch, clock.epoch - 1);
        assert_eq!(stake.delegation.deactivation_epoch, u64::MAX);
    } else {
        assert_eq!(onramp_stake, None);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum OnRampState {
    Initialized,
    Activating,
    Active,
    Deactive,
}

#[test_case(OnRampState::Initialized, true; "move_lamports_fresh")]
#[test_case(OnRampState::Activating, false; "topup_warm")]
#[test_case(OnRampState::Activating, true; "move_lamports_warm")]
#[test_case(OnRampState::Active, false; "reset_hot")]
#[test_case(OnRampState::Active, true; "move_lamports_hot")]
#[test_case(OnRampState::Deactive, true; "move_lamports_cold")]
#[tokio::test]
async fn move_value_success(onramp_state: OnRampState, move_lamports: bool) {
    let mut context = program_test(false).start_with_context().await;
    let accounts = SinglePoolAccounts::default();
    accounts
        .initialize_for_deposit(&mut context, TEST_STAKE_AMOUNT, None)
        .await;
    advance_epoch(&mut context).await;

    // active onramp can be as low as minimum_delegation but this is more convenient
    let lamports = get_minimum_pool_balance(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
    )
    .await;

    // set up an activating onramp
    if onramp_state >= OnRampState::Activating {
        transfer(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &accounts.onramp_account,
            lamports,
        )
        .await;

        replenish(&mut context, &accounts.vote_account.pubkey()).await;
    }

    // allow the delegation to activate
    if onramp_state >= OnRampState::Active {
        advance_epoch(&mut context).await;
    }

    // move it over; this case is inactive and behaves identical to Initialized
    if onramp_state == OnRampState::Deactive {
        replenish(&mut context, &accounts.vote_account.pubkey()).await;
    }

    // if we are testing the pool -> onramp leg, add lamports for it
    if move_lamports {
        transfer(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &accounts.stake_account,
            lamports,
        )
        .await;
    }

    // this one case is to test reupping an activating delegation
    if onramp_state == OnRampState::Activating && !move_lamports {
        transfer(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &accounts.onramp_account,
            lamports,
        )
        .await;
    }

    // this is the replenish we test
    replenish(&mut context, &accounts.vote_account.pubkey()).await;

    let clock = context.banks_client.get_sysvar::<Clock>().await.unwrap();
    let stake_history = context
        .banks_client
        .get_sysvar::<StakeHistory>()
        .await
        .unwrap();

    let (pool_meta, pool_stake, pool_lamports) =
        get_stake_account(&mut context.banks_client, &accounts.stake_account).await;
    let pool_status = pool_stake
        .unwrap()
        .delegation
        .stake_activating_and_deactivating(clock.epoch, &stake_history, Some(0));
    let pool_rent = pool_meta.rent_exempt_reserve;

    let (onramp_meta, onramp_stake, onramp_lamports) =
        get_stake_account(&mut context.banks_client, &accounts.onramp_account).await;
    let onramp_status = onramp_stake
        .map(|stake| {
            stake
                .delegation
                .stake_activating_and_deactivating(clock.epoch, &stake_history, Some(0))
        })
        .unwrap_or_default();
    let onramp_rent = onramp_meta.rent_exempt_reserve;

    match (onramp_state, move_lamports) {
        // stake moved already before test or because of test, new lamports were added to onramp
        (OnRampState::Deactive, true) | (OnRampState::Active, true) => {
            assert_eq!(pool_status.effective, lamports * 2);
            assert_eq!(pool_lamports, lamports * 2 + pool_rent);

            assert_eq!(onramp_status.effective, 0);
            assert_eq!(onramp_status.activating, lamports);
            assert_eq!(onramp_lamports, lamports + onramp_rent);
        }
        // no stake moved, but lamports did
        (OnRampState::Initialized, true) => {
            assert_eq!(pool_status.effective, lamports);
            assert_eq!(pool_lamports, lamports + pool_rent);

            assert_eq!(onramp_status.effective, 0);
            assert_eq!(onramp_status.activating, lamports);
            assert_eq!(onramp_lamports, lamports + onramp_rent);
        }
        // no excess lamports moved, just stake
        (OnRampState::Active, false) => {
            assert_eq!(pool_status.effective, lamports * 2);
            assert_eq!(pool_lamports, lamports * 2 + pool_rent);

            assert_eq!(onramp_status.effective, 0);
            assert_eq!(onramp_status.activating, 0);
            assert_eq!(onramp_lamports, onramp_rent);
        }
        // topped up an existing activation, either with pool or onramp lamports
        (OnRampState::Activating, _) => {
            assert_eq!(pool_status.effective, lamports);
            assert_eq!(pool_lamports, lamports + pool_rent);

            assert_eq!(onramp_status.effective, 0);
            assert_eq!(onramp_status.activating, lamports * 2);
            assert_eq!(onramp_lamports, lamports * 2 + onramp_rent);
        }
        // we have no further test cases
        _ => unreachable!(),
    }
}

#[test_case(true; "activated")]
#[test_case(false; "activating")]
#[tokio::test]
async fn fail_onramp_doesnt_exist(activate: bool) {
    let mut context = program_test(false).start_with_context().await;
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

    let mut instructions = instruction::initialize(
        &id(),
        &accounts.vote_account.pubkey(),
        &context.payer.pubkey(),
        &rent,
        minimum_pool_balance,
    );

    // guard against instruction moving in the builder function
    assert_eq!(&instructions[5].data, &[6]);
    let onramp_instruction = instructions.remove(5);

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&context.payer.pubkey()),
        &[&context.payer],
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

    // pool is now activating or active with no onramp account
    // replenish should fail because the onramp is required
    let replenish_instruction = instruction::replenish_pool(&id(), &accounts.vote_account.pubkey());
    let transaction = Transaction::new_signed_with_payer(
        &[replenish_instruction.clone()],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    let e = context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err();
    check_error(e, SinglePoolError::OnRampDoesntExist);

    // creating onramp lets replenish succeed in the same epoch
    let transaction = Transaction::new_signed_with_payer(
        &[onramp_instruction],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    refresh_blockhash(&mut context).await;
    let transaction = Transaction::new_signed_with_payer(
        &[replenish_instruction],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}
