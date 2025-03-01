#![allow(clippy::arithmetic_side_effects)]
#![cfg(feature = "test-sbf")]

mod helpers;

use {
    helpers::*,
    solana_program_test::*,
    solana_sdk::{
        account::AccountSharedData,
        signature::Signer,
        stake::{
            stake_flags::StakeFlags,
            state::{Delegation, Stake, StakeStateV2},
        },
        sysvar::clock::Clock,
        transaction::Transaction,
    },
    spl_single_pool::{error::SinglePoolError, id, instruction},
    test_case::test_case,
};

// NOTE we have no true/true case because the onramp can only be reactivated given an active pool
// this is by design to reduce complexity. DeactivateDelinquent should be rare so waiting an epoch is no burden
#[test_case(false, false; "noop")]
#[test_case(true, false; "pool")]
#[test_case(false, true; "onramp")]
#[tokio::test]
async fn reactivate_success(reactivate_pool: bool, activate_onramp: bool) {
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
    if activate_onramp {
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

    // replenish
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

    if activate_onramp {
        let stake = onramp_stake.unwrap();
        assert_eq!(stake.delegation.activation_epoch, clock.epoch - 1);
        assert_eq!(stake.delegation.deactivation_epoch, u64::MAX);
    } else {
        assert_eq!(onramp_stake, None);
    }
}

// XXX ok im awake again
// i did the activate test for both accounts
// fail test below gets deleted. we no longer hard error for that case
// * fail if onramp doesnt exist. need our own initialize for that
// * move stake for active onramp, move lamports for excess lamports
//   plus both at once. reactivate test captures the neither case
//   also ensure onramp goes back into activating status if new lamps
//   *or* if there *are* extra lamps and we *dont* move new ones
//   in other words we never mess up and leave stake behind
//   also cover the case of an already activating onramp that must be topped up

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
    check_error(e, SinglePoolError::OnrampDoesntExist);

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
