#![deny(missing_docs)]

//! A program for liquid staking with a single validator

pub mod error;
pub mod inline_mpl_token_metadata;
pub mod instruction;
pub mod processor;
pub mod state;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

use {solana_native_token::LAMPORTS_PER_SOL, solana_pubkey::Pubkey};

solana_pubkey::declare_id!("SVSPxpvHdN29nkVg9rPapPNDddN5DipNLRUFhyjFThE");

/// Fee charged for the `DepositSol` instruction. This fee may be adjusted down in
/// the future depending on how `DepositSol` is used in practice. Care should be
/// taken if using this number for calculations in third-party libraries or programs,
/// as it is not guaranteed to remain at this value.
pub const DEPOSIT_SOL_FEE_BPS: u64 = 100;
const MAX_BPS: u64 = 10_000;

const POOL_PREFIX: &[u8] = b"pool";
const POOL_STAKE_PREFIX: &[u8] = b"stake";
const POOL_ONRAMP_PREFIX: &[u8] = b"onramp";
const POOL_MINT_PREFIX: &[u8] = b"mint";
const POOL_MINT_AUTHORITY_PREFIX: &[u8] = b"mint_authority";
const POOL_STAKE_AUTHORITY_PREFIX: &[u8] = b"stake_authority";
const POOL_MPL_AUTHORITY_PREFIX: &[u8] = b"mpl_authority";

const PHANTOM_TOKEN_AMOUNT: u64 = LAMPORTS_PER_SOL;
const MINT_DECIMALS: u8 = 9;
const PERPETUAL_NEW_WARMUP_COOLDOWN_RATE_EPOCH: Option<u64> = Some(0);

const VOTE_STATE_DISCRIMINATOR_END: usize = 4;
const VOTE_STATE_AUTHORIZED_WITHDRAWER_START: usize = 36;
const VOTE_STATE_AUTHORIZED_WITHDRAWER_END: usize = 68;

fn find_pool_address_and_bump(program_id: &Pubkey, vote_account_address: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[POOL_PREFIX, vote_account_address.as_ref()], program_id)
}

fn find_pool_stake_address_and_bump(program_id: &Pubkey, pool_address: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[POOL_STAKE_PREFIX, pool_address.as_ref()], program_id)
}

fn find_pool_onramp_address_and_bump(program_id: &Pubkey, pool_address: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[POOL_ONRAMP_PREFIX, pool_address.as_ref()], program_id)
}

fn find_pool_mint_address_and_bump(program_id: &Pubkey, pool_address: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[POOL_MINT_PREFIX, pool_address.as_ref()], program_id)
}

fn find_pool_stake_authority_address_and_bump(
    program_id: &Pubkey,
    pool_address: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[POOL_STAKE_AUTHORITY_PREFIX, pool_address.as_ref()],
        program_id,
    )
}

fn find_pool_mint_authority_address_and_bump(
    program_id: &Pubkey,
    pool_address: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[POOL_MINT_AUTHORITY_PREFIX, pool_address.as_ref()],
        program_id,
    )
}

fn find_pool_mpl_authority_address_and_bump(
    program_id: &Pubkey,
    pool_address: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[POOL_MPL_AUTHORITY_PREFIX, pool_address.as_ref()],
        program_id,
    )
}

/// Find the canonical pool address for a given vote account.
pub fn find_pool_address(program_id: &Pubkey, vote_account_address: &Pubkey) -> Pubkey {
    find_pool_address_and_bump(program_id, vote_account_address).0
}

/// Find the canonical main stake account address for a given pool account.
pub fn find_pool_stake_address(program_id: &Pubkey, pool_address: &Pubkey) -> Pubkey {
    find_pool_stake_address_and_bump(program_id, pool_address).0
}

/// Find the canonical stake on-ramp account address for a given pool account.
pub fn find_pool_onramp_address(program_id: &Pubkey, pool_address: &Pubkey) -> Pubkey {
    find_pool_onramp_address_and_bump(program_id, pool_address).0
}

/// Find the canonical token mint address for a given pool account.
pub fn find_pool_mint_address(program_id: &Pubkey, pool_address: &Pubkey) -> Pubkey {
    find_pool_mint_address_and_bump(program_id, pool_address).0
}

/// Find the canonical stake authority address for a given pool account.
pub fn find_pool_stake_authority_address(program_id: &Pubkey, pool_address: &Pubkey) -> Pubkey {
    find_pool_stake_authority_address_and_bump(program_id, pool_address).0
}

/// Find the canonical mint authority address for a given pool account.
pub fn find_pool_mint_authority_address(program_id: &Pubkey, pool_address: &Pubkey) -> Pubkey {
    find_pool_mint_authority_address_and_bump(program_id, pool_address).0
}

/// Find the canonical MPL authority address for a given pool account.
pub fn find_pool_mpl_authority_address(program_id: &Pubkey, pool_address: &Pubkey) -> Pubkey {
    find_pool_mpl_authority_address_and_bump(program_id, pool_address).0
}
