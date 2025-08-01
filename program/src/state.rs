//! State transition types

use {
    crate::{error::SinglePoolError, find_pool_address},
    borsh::{BorshDeserialize, BorshSchema, BorshSerialize},
    solana_account_info::AccountInfo,
    solana_borsh::v1::try_from_slice_unchecked,
    solana_program_error::ProgramError,
    solana_pubkey::Pubkey,
};

/// Single-Validator Stake Pool account type
#[derive(Clone, Debug, Default, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum SinglePoolAccountType {
    /// Uninitialized account
    #[default]
    Uninitialized,
    /// Main pool account
    Pool,
}

/// Single-Validator Stake Pool account, used to derive all PDAs
#[derive(Clone, Debug, Default, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct SinglePool {
    /// Pool account type, reserved for future compatibility
    pub account_type: SinglePoolAccountType,
    /// The vote account this pool is mapped to
    pub vote_account_address: Pubkey,
}
impl SinglePool {
    /// Create a `SinglePool` struct from its account info
    pub fn from_account_info(
        account_info: &AccountInfo,
        program_id: &Pubkey,
    ) -> Result<Self, ProgramError> {
        // pool is allocated and owned by this program
        if account_info.data_len() == 0 || account_info.owner != program_id {
            return Err(SinglePoolError::InvalidPoolAccount.into());
        }

        let pool = try_from_slice_unchecked::<SinglePool>(&account_info.data.borrow())?;

        // pool is well-typed
        if pool.account_type != SinglePoolAccountType::Pool {
            return Err(SinglePoolError::InvalidPoolAccount.into());
        }

        // pool vote account address is properly configured. in practice this is
        // irrefutable because the pool is initialized from the address that
        // derives it, and never modified
        if *account_info.key != find_pool_address(program_id, &pool.vote_account_address) {
            return Err(SinglePoolError::InvalidPoolAccount.into());
        }

        Ok(pool)
    }
}
