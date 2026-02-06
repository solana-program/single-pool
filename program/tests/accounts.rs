#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::items_after_test_module)]

mod helpers;

use {
    helpers::*,
    solana_program_test::*,
    solana_pubkey::pubkey,
    solana_sdk::{
        instruction::Instruction, program_error::ProgramError, pubkey::Pubkey, signature::Signer,
        transaction::Transaction,
    },
    solana_stake_interface::program as stake_program,
    solana_system_interface::program as system_program,
    spl_single_pool::{
        error::SinglePoolError,
        find_pool_onramp_address, id,
        instruction::{self, SinglePoolInstruction},
    },
    spl_token_interface as spl_token,
    test_case::test_matrix,
};

#[derive(Clone, Debug, PartialEq, Eq)]
enum TestMode {
    Initialize,
    Deposit,
    Withdraw,
}

// build a full transaction for initialize, deposit, and withdraw
// this is used to test knocking out individual accounts, for the sake of
// confirming the pubkeys are checked
async fn build_instructions(
    context: &mut ProgramTestContext,
    accounts: &SinglePoolAccounts,
    test_mode: TestMode,
    remove_onramp: bool,
) -> (Vec<Instruction>, usize) {
    let initialize_instructions = if test_mode == TestMode::Initialize {
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

        transfer(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
            &accounts.alice.pubkey(),
            USER_STARTING_LAMPORTS,
        )
        .await;

        let rent = context.banks_client.get_rent().await.unwrap();
        let minimum_pool_balance = get_minimum_pool_balance(
            &mut context.banks_client,
            &context.payer,
            &context.last_blockhash,
        )
        .await;

        instruction::initialize(
            &id(),
            &accounts.vote_account.pubkey(),
            &accounts.alice.pubkey(),
            &rent,
            minimum_pool_balance,
        )
    } else {
        accounts
            .initialize_for_deposit(context, TEST_STAKE_AMOUNT, None)
            .await;
        advance_epoch(context).await;

        vec![]
    };

    let mut deposit_instructions = instruction::deposit(
        &id(),
        &accounts.pool,
        &accounts.alice_stake.pubkey(),
        &accounts.alice_token,
        &accounts.alice.pubkey(),
        &accounts.alice.pubkey(),
    );

    let mut withdraw_instructions = if test_mode == TestMode::Withdraw {
        let transaction = Transaction::new_signed_with_payer(
            &deposit_instructions,
            Some(&accounts.alice.pubkey()),
            &[&accounts.alice],
            context.last_blockhash,
        );

        context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        create_blank_stake_account(
            &mut context.banks_client,
            &context.payer,
            &accounts.alice,
            &context.last_blockhash,
            &accounts.alice_stake,
        )
        .await;

        instruction::withdraw(
            &id(),
            &accounts.pool,
            &accounts.alice_stake.pubkey(),
            &accounts.alice.pubkey(),
            &accounts.alice_token,
            &accounts.alice.pubkey(),
            get_token_balance(&mut context.banks_client, &accounts.alice_token).await,
        )
    } else {
        vec![]
    };

    if remove_onramp {
        let instruction = match test_mode {
            TestMode::Deposit => deposit_instructions.last_mut().unwrap(),
            TestMode::Withdraw => withdraw_instructions.last_mut().unwrap(),
            TestMode::Initialize => unreachable!(),
        };

        assert_eq!(instruction.accounts[2].pubkey, accounts.onramp_account);
        instruction.accounts.remove(2);
    }

    // ints hardcoded to guard against instructions moving with code changes
    // if these asserts fail, update them to match the new multi-instruction builders
    let (instructions, index, enum_tag) = match test_mode {
        TestMode::Initialize => (initialize_instructions, 4, 0),
        TestMode::Deposit => (deposit_instructions, 2, 2),
        TestMode::Withdraw => (withdraw_instructions, 1, 3),
    };

    assert_eq!(instructions[index].program_id, id());
    assert_eq!(instructions[index].data[0], enum_tag);

    (instructions, index)
}

// test that account addresses are checked properly
#[test_matrix(
    [StakeProgramVersion::Live, StakeProgramVersion::Upcoming, StakeProgramVersion::Testing],
    [TestMode::Initialize, TestMode::Deposit, TestMode::Withdraw],
    [false, true]
)]
#[tokio::test]
async fn fail_account_checks(
    stake_version: StakeProgramVersion,
    test_mode: TestMode,
    remove_onramp: bool,
) {
    // initialize does not take the onramp account
    if test_mode == TestMode::Initialize && remove_onramp {
        return;
    }

    let Some(program_test) = program_test(stake_version) else {
        return;
    };
    let mut context = program_test.start_with_context().await;

    let accounts = SinglePoolAccounts::default();
    let (instructions, i) =
        build_instructions(&mut context, &accounts, test_mode, remove_onramp).await;
    let bad_pubkey = pubkey!("BAD1111111111111111111111111111111111111111");

    for j in 0..instructions[i].accounts.len() {
        let mut instructions = instructions.clone();
        let instruction_pubkey = instructions[i].accounts[j].pubkey;

        // wallet address can be arbitrary
        if instruction_pubkey == accounts.alice.pubkey() {
            continue;
        }

        // while onramp is optional, an incorrect onramp misaligns all subsequent accounts
        // this is not a problem for the program and causes the mint to fail to validate, but requires tweaking this test
        if !remove_onramp && instruction_pubkey == accounts.pool {
            if let Some(onramp_account) = instructions[i]
                .accounts
                .iter_mut()
                .find(|account| account.pubkey == accounts.onramp_account)
            {
                onramp_account.pubkey = find_pool_onramp_address(&id(), &bad_pubkey);
            }
        }

        instructions[i].accounts[j].pubkey = bad_pubkey;

        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&accounts.alice.pubkey()),
            &[&accounts.alice],
            context.last_blockhash,
        );

        // random addresses should error in some way
        let e = context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err();

        // these specific accounts we can also make sure we hit the explicit check, before we use it
        if instruction_pubkey == accounts.pool {
            check_error(e, SinglePoolError::InvalidPoolAccount)
        } else if instruction_pubkey == accounts.stake_account {
            check_error(e, SinglePoolError::InvalidPoolStakeAccount)
        } else if instruction_pubkey == accounts.onramp_account {
            // NOTE add onramp error here when onramp is mandatory
        } else if instruction_pubkey == accounts.stake_authority {
            check_error(e, SinglePoolError::InvalidPoolStakeAuthority)
        } else if instruction_pubkey == accounts.mint_authority {
            check_error(e, SinglePoolError::InvalidPoolMintAuthority)
        } else if instruction_pubkey == accounts.mpl_authority {
            check_error(e, SinglePoolError::InvalidPoolMplAuthority)
        } else if instruction_pubkey == accounts.mint {
            check_error(e, SinglePoolError::InvalidPoolMint)
        } else if [system_program::id(), spl_token::id(), stake_program::id()]
            .contains(&instruction_pubkey)
        {
            check_error(e, ProgramError::IncorrectProgramId)
        }
    }

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&accounts.alice.pubkey()),
        &[&accounts.alice],
        context.last_blockhash,
    );

    // sanity check the unmodified transaction does work
    context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}

// make an individual instruction for all program instructions
// the match is just so this will error if new instructions are added
// if you are reading this because of that error, add the case to the
// `consistent_account_order` test!!!
fn make_basic_instruction(
    accounts: &SinglePoolAccounts,
    instruction_type: SinglePoolInstruction,
) -> Instruction {
    match instruction_type {
        SinglePoolInstruction::InitializePool => {
            instruction::initialize_pool(&id(), &accounts.vote_account.pubkey())
        }
        SinglePoolInstruction::ReplenishPool => {
            instruction::replenish_pool(&id(), &accounts.vote_account.pubkey())
        }
        SinglePoolInstruction::DepositStake => instruction::deposit_stake(
            &id(),
            &accounts.pool,
            &Pubkey::default(),
            &Pubkey::default(),
            &Pubkey::default(),
        ),
        SinglePoolInstruction::WithdrawStake { .. } => instruction::withdraw_stake(
            &id(),
            &accounts.pool,
            &Pubkey::default(),
            &Pubkey::default(),
            &Pubkey::default(),
            0,
        ),
        SinglePoolInstruction::CreateTokenMetadata => {
            instruction::create_token_metadata(&id(), &accounts.pool, &Pubkey::default())
        }
        SinglePoolInstruction::UpdateTokenMetadata { .. } => instruction::update_token_metadata(
            &id(),
            &accounts.vote_account.pubkey(),
            &accounts.withdrawer.pubkey(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
        ),
        SinglePoolInstruction::InitializePoolOnRamp => {
            instruction::initialize_pool_onramp(&id(), &accounts.pool)
        }
    }
}

// advanced technology
fn is_sorted<T>(data: &[T]) -> bool
where
    T: Ord,
{
    data.windows(2).all(|w| w[0] <= w[1])
}

// check that major accounts always show up in the same order, to spare
// developer confusion
#[test]
fn consistent_account_order() {
    let accounts = SinglePoolAccounts::default();

    let ordering = vec![
        accounts.vote_account.pubkey(),
        accounts.pool,
        accounts.stake_account,
        accounts.onramp_account,
        accounts.mint,
        accounts.stake_authority,
        accounts.mint_authority,
        accounts.mpl_authority,
    ];

    let instructions = vec![
        make_basic_instruction(&accounts, SinglePoolInstruction::InitializePool),
        make_basic_instruction(&accounts, SinglePoolInstruction::ReplenishPool),
        make_basic_instruction(&accounts, SinglePoolInstruction::DepositStake),
        make_basic_instruction(
            &accounts,
            SinglePoolInstruction::WithdrawStake {
                user_stake_authority: Pubkey::default(),
                token_amount: 0,
            },
        ),
        make_basic_instruction(&accounts, SinglePoolInstruction::CreateTokenMetadata),
        make_basic_instruction(
            &accounts,
            SinglePoolInstruction::UpdateTokenMetadata {
                name: "".to_string(),
                symbol: "".to_string(),
                uri: "".to_string(),
            },
        ),
        make_basic_instruction(&accounts, SinglePoolInstruction::InitializePoolOnRamp),
    ];

    for instruction in instructions {
        let mut indexes = vec![];

        for target in &ordering {
            if let Some(i) = instruction
                .accounts
                .iter()
                .position(|meta| meta.pubkey == *target)
            {
                indexes.push(i);
            }
        }

        assert!(is_sorted(&indexes));
    }
}
