//! Instruction types

#![allow(clippy::too_many_arguments)]

use {
    crate::{
        find_default_deposit_account_address_and_seed, find_pool_address, find_pool_mint_address,
        find_pool_mint_authority_address, find_pool_mpl_authority_address,
        find_pool_onramp_address, find_pool_stake_address, find_pool_stake_authority_address,
        inline_mpl_token_metadata::{self, pda::find_metadata_account},
        state::SinglePool,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    solana_instruction::{AccountMeta, Instruction},
    solana_program_pack::Pack,
    solana_pubkey::Pubkey,
    solana_rent::Rent,
    solana_stake_interface as stake,
    solana_system_interface::{instruction as system_instruction, program as system_program},
    solana_sysvar as sysvar,
};

/// Instructions supported by the `SinglePool` program.
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum SinglePoolInstruction {
    ///   Initialize the mint and main stake account for a new single-validator
    ///   stake pool. The pool stake account must contain the rent-exempt
    ///   minimum plus the minimum balance of 1 sol. No tokens will be minted;
    ///   to deposit more, use `Deposit` after `InitializeStake`.
    ///
    ///   0. `[]` Validator vote account
    ///   1. `[w]` Pool account
    ///   2. `[w]` Pool stake account
    ///   3. `[w]` Pool token mint
    ///   4. `[]` Pool stake authority
    ///   5. `[]` Pool mint authority
    ///   6. `[]` Rent sysvar
    ///   7. `[]` Clock sysvar
    ///   8. `[]` Stake history sysvar
    ///   9. `[]` Stake config sysvar
    ///  10. `[]` System program
    ///  11. `[]` Token program
    ///  12. `[]` Stake program
    InitializePool,

    ///   Bring the pool stake accounts to their optimal state. This performs
    ///   several operations:
    ///   * If the main stake account has been deactivated by
    ///     `DeactivateDelinquent`, reactivate it.
    ///   * Then, if the main stake account is already fully active:
    ///     - If the on-ramp is fully active, move its stake to the main account.
    ///     - If the main account has excess lamports, move them to the on-ramp.
    ///     - Delegate the on-ramp if it has excess lamports to activate.
    ///
    ///   Combined, these operations allow harvesting and delegating MEV rewards
    ///   and will eventually allow depositing liquid sol for pool tokens.
    ///
    ///   This instruction is idempotent and gracefully skips operations that
    ///   would fail or have no effect, up to no-op. This allows it to be
    ///   executed speculatively or as part of arbitrary flows involving the pool.
    ///   If the on-ramp account is already activating, and there are excess lamports
    ///   beyond the activating delegation, it increases the delegation to include them.
    ///
    ///   This instruction will fail with an error if the on-ramp account does not
    ///   exist. If the pool does not have the account, `InitializePoolOnRamp` must
    ///   be called to create it.
    ///
    ///   0. `[]` Validator vote account
    ///   1. `[]` Pool account
    ///   2. `[w]` Pool stake account
    ///   3. `[w]` Pool on-ramp account
    ///   4. `[]` Pool stake authority
    ///   5. `[]` Clock sysvar
    ///   6. `[]` Stake history sysvar
    ///   7. `[]` Stake config sysvar
    ///   8. `[]` Stake program
    ReplenishPool,

    ///   Deposit stake into the pool. The output is a "pool" token
    ///   representing fractional ownership of the pool stake. Inputs are
    ///   converted to the current ratio.
    ///
    ///   0. `[]` Pool account
    ///   1. `[w]` Pool stake account
    ///   2. `[w]` Pool token mint
    ///   3. `[]` Pool stake authority
    ///   4. `[]` Pool mint authority
    ///   5. `[w]` User stake account to join to the pool
    ///   6. `[w]` User account to receive pool tokens
    ///   7. `[w]` User account to receive lamports
    ///   8. `[]` Clock sysvar
    ///   9. `[]` Stake history sysvar
    ///  10. `[]` Token program
    ///  11. `[]` Stake program
    DepositStake,

    ///   Redeem tokens issued by this pool for stake at the current ratio.
    ///
    ///   0. `[]` Pool account
    ///   1. `[w]` Pool stake account
    ///   2. `[w]` Pool token mint
    ///   3. `[]` Pool stake authority
    ///   4. `[]` Pool mint authority
    ///   5. `[w]` User stake account to receive stake at
    ///   6. `[w]` User account to take pool tokens from
    ///   7. `[]` Clock sysvar
    ///   8. `[]` Token program
    ///   9. `[]` Stake program
    WithdrawStake {
        /// User authority for the new stake account
        user_stake_authority: Pubkey,
        /// Amount of tokens to redeem for stake
        token_amount: u64,
    },

    ///   Create token metadata for the stake-pool token in the metaplex-token
    ///   program. Step three of the permissionless three-stage initialization
    ///   flow.
    ///   Note this instruction is not necessary for the pool to operate, to
    ///   ensure we cannot be broken by upstream.
    ///
    ///   0. `[]` Pool account
    ///   1. `[]` Pool token mint
    ///   2. `[]` Pool mint authority
    ///   3. `[]` Pool MPL authority
    ///   4. `[s, w]` Payer for creation of token metadata account
    ///   5. `[w]` Token metadata account
    ///   6. `[]` Metadata program id
    ///   7. `[]` System program id
    CreateTokenMetadata,

    ///   Update token metadata for the stake-pool token in the metaplex-token
    ///   program.
    ///
    ///   0. `[]` Validator vote account
    ///   1. `[]` Pool account
    ///   2. `[]` Pool MPL authority
    ///   3. `[s]` Vote account authorized withdrawer
    ///   4. `[w]` Token metadata account
    ///   5. `[]` Metadata program id
    UpdateTokenMetadata {
        /// Token name
        name: String,
        /// Token symbol e.g. `stkSOL`
        symbol: String,
        /// URI of the uploaded metadata of the spl-token
        uri: String,
    },

    ///   Create the on-ramp account for a single-validator stake pool, which
    ///   is used to delegate liquid sol so that it can be merged into the main
    ///   pool account as active stake.
    ///
    ///   New pools created with `initialize()` will include this instruction
    ///   automatically. Existing pools must use `InitializePoolOnRamp` to upgrade to
    ///   the latest version.
    ///
    ///   This is a temporary instruction that will be deprecated some time after all
    ///   existing pools have upgraded. Its logic is intended to be incorporated
    ///   into `InitializePool` itself.
    ///
    ///   0. `[]` Pool account
    ///   1. `[w]` Pool on-ramp account
    ///   2. `[]` Pool stake authority
    ///   3. `[]` Rent sysvar
    ///   4. `[]` System program
    ///   5. `[]` Stake program
    InitializePoolOnRamp,
}

/// Creates all necessary instructions to initialize the stake pool.
pub fn initialize(
    program_id: &Pubkey,
    vote_account_address: &Pubkey,
    payer: &Pubkey,
    rent: &Rent,
    minimum_pool_balance: u64,
) -> Vec<Instruction> {
    let pool_address = find_pool_address(program_id, vote_account_address);
    let pool_rent = rent.minimum_balance(std::mem::size_of::<SinglePool>());

    let stake_address = find_pool_stake_address(program_id, &pool_address);
    let onramp_address = find_pool_onramp_address(program_id, &pool_address);
    let stake_space = std::mem::size_of::<stake::state::StakeStateV2>();
    let stake_rent = rent.minimum_balance(stake_space);
    let stake_rent_plus_minimum = stake_rent.saturating_add(minimum_pool_balance);

    let mint_address = find_pool_mint_address(program_id, &pool_address);
    let mint_rent = rent.minimum_balance(spl_token::state::Mint::LEN);

    vec![
        system_instruction::transfer(payer, &pool_address, pool_rent),
        system_instruction::transfer(payer, &stake_address, stake_rent_plus_minimum),
        system_instruction::transfer(payer, &onramp_address, stake_rent),
        system_instruction::transfer(payer, &mint_address, mint_rent),
        initialize_pool(program_id, vote_account_address),
        initialize_pool_onramp(program_id, &pool_address),
        create_token_metadata(program_id, &pool_address, payer),
    ]
}

/// Creates an `InitializePool` instruction.
pub fn initialize_pool(program_id: &Pubkey, vote_account_address: &Pubkey) -> Instruction {
    let pool_address = find_pool_address(program_id, vote_account_address);
    let mint_address = find_pool_mint_address(program_id, &pool_address);

    let data = borsh::to_vec(&SinglePoolInstruction::InitializePool).unwrap();
    let accounts = vec![
        AccountMeta::new_readonly(*vote_account_address, false),
        AccountMeta::new(pool_address, false),
        AccountMeta::new(find_pool_stake_address(program_id, &pool_address), false),
        AccountMeta::new(mint_address, false),
        AccountMeta::new_readonly(
            find_pool_stake_authority_address(program_id, &pool_address),
            false,
        ),
        AccountMeta::new_readonly(
            find_pool_mint_authority_address(program_id, &pool_address),
            false,
        ),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::stake_history::id(), false),
        #[allow(deprecated)]
        AccountMeta::new_readonly(stake::config::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(stake::program::id(), false),
    ];

    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

/// Creates a `ReplenishPool` instruction.
pub fn replenish_pool(program_id: &Pubkey, vote_account_address: &Pubkey) -> Instruction {
    let pool_address = find_pool_address(program_id, vote_account_address);

    let data = borsh::to_vec(&SinglePoolInstruction::ReplenishPool).unwrap();
    let accounts = vec![
        AccountMeta::new_readonly(*vote_account_address, false),
        AccountMeta::new_readonly(pool_address, false),
        AccountMeta::new(find_pool_stake_address(program_id, &pool_address), false),
        AccountMeta::new(find_pool_onramp_address(program_id, &pool_address), false),
        AccountMeta::new_readonly(
            find_pool_stake_authority_address(program_id, &pool_address),
            false,
        ),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::stake_history::id(), false),
        #[allow(deprecated)]
        AccountMeta::new_readonly(stake::config::id(), false),
        AccountMeta::new_readonly(stake::program::id(), false),
    ];

    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

/// Creates all necessary instructions to deposit stake.
pub fn deposit(
    program_id: &Pubkey,
    pool_address: &Pubkey,
    user_stake_account: &Pubkey,
    user_token_account: &Pubkey,
    user_lamport_account: &Pubkey,
    user_withdraw_authority: &Pubkey,
) -> Vec<Instruction> {
    let pool_stake_authority = find_pool_stake_authority_address(program_id, pool_address);

    vec![
        stake::instruction::authorize(
            user_stake_account,
            user_withdraw_authority,
            &pool_stake_authority,
            stake::state::StakeAuthorize::Staker,
            None,
        ),
        stake::instruction::authorize(
            user_stake_account,
            user_withdraw_authority,
            &pool_stake_authority,
            stake::state::StakeAuthorize::Withdrawer,
            None,
        ),
        deposit_stake(
            program_id,
            pool_address,
            user_stake_account,
            user_token_account,
            user_lamport_account,
        ),
    ]
}

/// Creates a `DepositStake` instruction.
pub fn deposit_stake(
    program_id: &Pubkey,
    pool_address: &Pubkey,
    user_stake_account: &Pubkey,
    user_token_account: &Pubkey,
    user_lamport_account: &Pubkey,
) -> Instruction {
    let data = borsh::to_vec(&SinglePoolInstruction::DepositStake).unwrap();

    let accounts = vec![
        AccountMeta::new_readonly(*pool_address, false),
        AccountMeta::new(find_pool_stake_address(program_id, pool_address), false),
        AccountMeta::new(find_pool_mint_address(program_id, pool_address), false),
        AccountMeta::new_readonly(
            find_pool_stake_authority_address(program_id, pool_address),
            false,
        ),
        AccountMeta::new_readonly(
            find_pool_mint_authority_address(program_id, pool_address),
            false,
        ),
        AccountMeta::new(*user_stake_account, false),
        AccountMeta::new(*user_token_account, false),
        AccountMeta::new(*user_lamport_account, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::stake_history::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(stake::program::id(), false),
    ];

    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

/// Creates all necessary instructions to withdraw stake into a given stake
/// account. If a new stake account is required, the user should first include
/// `system_instruction::create_account` with account size
/// `std::mem::size_of::<stake::state::StakeStateV2>()` and owner
/// `stake::program::id()`.
pub fn withdraw(
    program_id: &Pubkey,
    pool_address: &Pubkey,
    user_stake_account: &Pubkey,
    user_stake_authority: &Pubkey,
    user_token_account: &Pubkey,
    user_token_authority: &Pubkey,
    token_amount: u64,
) -> Vec<Instruction> {
    vec![
        spl_token::instruction::approve(
            &spl_token::id(),
            user_token_account,
            &find_pool_mint_authority_address(program_id, pool_address),
            user_token_authority,
            &[],
            token_amount,
        )
        .unwrap(),
        withdraw_stake(
            program_id,
            pool_address,
            user_stake_account,
            user_stake_authority,
            user_token_account,
            token_amount,
        ),
    ]
}

/// Creates a `WithdrawStake` instruction.
pub fn withdraw_stake(
    program_id: &Pubkey,
    pool_address: &Pubkey,
    user_stake_account: &Pubkey,
    user_stake_authority: &Pubkey,
    user_token_account: &Pubkey,
    token_amount: u64,
) -> Instruction {
    let data = borsh::to_vec(&SinglePoolInstruction::WithdrawStake {
        user_stake_authority: *user_stake_authority,
        token_amount,
    })
    .unwrap();

    let accounts = vec![
        AccountMeta::new_readonly(*pool_address, false),
        AccountMeta::new(find_pool_stake_address(program_id, pool_address), false),
        AccountMeta::new(find_pool_mint_address(program_id, pool_address), false),
        AccountMeta::new_readonly(
            find_pool_stake_authority_address(program_id, pool_address),
            false,
        ),
        AccountMeta::new_readonly(
            find_pool_mint_authority_address(program_id, pool_address),
            false,
        ),
        AccountMeta::new(*user_stake_account, false),
        AccountMeta::new(*user_token_account, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(stake::program::id(), false),
    ];

    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

/// Creates necessary instructions to create and delegate a new stake account to
/// a given validator. Uses a fixed address for each wallet and vote account
/// combination to make it easier to find for deposits. This is an optional
/// helper function; deposits can come from any owned stake account without
/// lockup.
pub fn create_and_delegate_user_stake(
    program_id: &Pubkey,
    vote_account_address: &Pubkey,
    user_wallet: &Pubkey,
    rent: &Rent,
    stake_amount: u64,
) -> Vec<Instruction> {
    let pool_address = find_pool_address(program_id, vote_account_address);
    let stake_space = std::mem::size_of::<stake::state::StakeStateV2>();
    let lamports = rent
        .minimum_balance(stake_space)
        .saturating_add(stake_amount);
    let (deposit_address, deposit_seed) =
        find_default_deposit_account_address_and_seed(&pool_address, user_wallet);

    stake::instruction::create_account_with_seed_and_delegate_stake(
        user_wallet,
        &deposit_address,
        user_wallet,
        &deposit_seed,
        vote_account_address,
        &stake::state::Authorized::auto(user_wallet),
        &stake::state::Lockup::default(),
        lamports,
    )
}

/// Creates a `CreateTokenMetadata` instruction.
pub fn create_token_metadata(
    program_id: &Pubkey,
    pool_address: &Pubkey,
    payer: &Pubkey,
) -> Instruction {
    let pool_mint = find_pool_mint_address(program_id, pool_address);
    let (token_metadata, _) = find_metadata_account(&pool_mint);
    let data = borsh::to_vec(&SinglePoolInstruction::CreateTokenMetadata).unwrap();

    let accounts = vec![
        AccountMeta::new_readonly(*pool_address, false),
        AccountMeta::new_readonly(pool_mint, false),
        AccountMeta::new_readonly(
            find_pool_mint_authority_address(program_id, pool_address),
            false,
        ),
        AccountMeta::new_readonly(
            find_pool_mpl_authority_address(program_id, pool_address),
            false,
        ),
        AccountMeta::new(*payer, true),
        AccountMeta::new(token_metadata, false),
        AccountMeta::new_readonly(inline_mpl_token_metadata::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

/// Creates an `UpdateTokenMetadata` instruction.
pub fn update_token_metadata(
    program_id: &Pubkey,
    vote_account_address: &Pubkey,
    authorized_withdrawer: &Pubkey,
    name: String,
    symbol: String,
    uri: String,
) -> Instruction {
    let pool_address = find_pool_address(program_id, vote_account_address);
    let pool_mint = find_pool_mint_address(program_id, &pool_address);
    let (token_metadata, _) = find_metadata_account(&pool_mint);
    let data =
        borsh::to_vec(&SinglePoolInstruction::UpdateTokenMetadata { name, symbol, uri }).unwrap();

    let accounts = vec![
        AccountMeta::new_readonly(*vote_account_address, false),
        AccountMeta::new_readonly(pool_address, false),
        AccountMeta::new_readonly(
            find_pool_mpl_authority_address(program_id, &pool_address),
            false,
        ),
        AccountMeta::new_readonly(*authorized_withdrawer, true),
        AccountMeta::new(token_metadata, false),
        AccountMeta::new_readonly(inline_mpl_token_metadata::id(), false),
    ];

    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

/// Creates a `InitializePoolOnRamp` instruction.
pub fn initialize_pool_onramp(program_id: &Pubkey, pool_address: &Pubkey) -> Instruction {
    let data = borsh::to_vec(&SinglePoolInstruction::InitializePoolOnRamp).unwrap();
    let accounts = vec![
        AccountMeta::new_readonly(*pool_address, false),
        AccountMeta::new(find_pool_onramp_address(program_id, pool_address), false),
        AccountMeta::new_readonly(
            find_pool_stake_authority_address(program_id, pool_address),
            false,
        ),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(stake::program::id(), false),
    ];

    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

/// Creates a `InitializePoolOnRamp` instruction plus the transfer to fund it.
/// This is for convenience, for users who need to create an on-ramp for existing pools.
/// We don't use it internally, because `initialize()` carries the necessary logic.
pub fn create_pool_onramp(
    program_id: &Pubkey,
    pool_address: &Pubkey,
    payer: &Pubkey,
    rent: &Rent,
) -> Vec<Instruction> {
    let onramp_address = find_pool_onramp_address(program_id, pool_address);
    let stake_space = std::mem::size_of::<stake::state::StakeStateV2>();
    let stake_rent = rent.minimum_balance(stake_space);

    vec![
        system_instruction::transfer(payer, &onramp_address, stake_rent),
        initialize_pool_onramp(program_id, pool_address),
    ]
}
