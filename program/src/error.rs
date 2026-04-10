//! Error types

use {
    solana_program_error::{ProgramError, ToStr},
    thiserror::Error,
};

/// Errors that may be returned by the `SinglePool` program.
#[derive(
    Clone, Debug, Eq, Error, num_enum::TryFromPrimitive, num_derive::FromPrimitive, PartialEq,
)]
#[repr(u32)]
pub enum SinglePoolError {
    // 0.
    /// Provided pool account has the wrong address for its vote account, is
    /// uninitialized, or otherwise invalid.
    #[error("InvalidPoolAccount")]
    InvalidPoolAccount,
    /// Provided pool stake account does not match address derived from the pool
    /// account.
    #[error("InvalidPoolStakeAccount")]
    InvalidPoolStakeAccount,
    /// Provided pool mint does not match address derived from the pool account.
    #[error("InvalidPoolMint")]
    InvalidPoolMint,
    /// Provided pool stake authority does not match address derived from the
    /// pool account.
    #[error("InvalidPoolStakeAuthority")]
    InvalidPoolStakeAuthority,
    /// Provided pool mint authority does not match address derived from the
    /// pool account.
    #[error("InvalidPoolMintAuthority")]
    InvalidPoolMintAuthority,

    // 5.
    /// Provided pool MPL authority does not match address derived from the pool
    /// account.
    #[error("InvalidPoolMplAuthority")]
    InvalidPoolMplAuthority,
    /// Provided metadata account does not match metadata account derived for
    /// pool mint.
    #[error("InvalidMetadataAccount")]
    InvalidMetadataAccount,
    /// Authorized withdrawer provided for metadata update does not match the
    /// vote account.
    #[error("InvalidMetadataSigner")]
    InvalidMetadataSigner,
    /// Not enough lamports provided for deposit to result in one pool token.
    #[error("DepositTooSmall")]
    DepositTooSmall,
    /// Not enough pool tokens provided to withdraw stake worth one lamport.
    #[error("WithdrawalTooSmall")]
    WithdrawalTooSmall,

    // 10
    /// Not enough stake to cover the provided quantity of pool tokens.
    /// This typically means the value exists in the pool as activating stake,
    /// and an epoch is required for it to become available. Otherwise, it means
    /// active stake in the on-ramp must be moved via `ReplenishPool`.
    #[error("WithdrawalTooLarge")]
    WithdrawalTooLarge,
    /// Required signature is missing.
    #[error("SignatureMissing")]
    SignatureMissing,
    /// Stake account is not in the state expected by the program.
    #[error("WrongStakeState")]
    WrongStakeState,
    /// Unsigned subtraction crossed the zero.
    #[error("ArithmeticOverflow")]
    ArithmeticOverflow,
    /// A calculation failed unexpectedly.
    /// (This error should never be surfaced; it stands in for failure
    /// conditions that should never be reached.)
    #[error("UnexpectedMathError")]
    UnexpectedMathError,

    // 15
    /// The `V0_23_5` vote account type is unsupported and should be upgraded via
    /// `convert_to_current()`.
    #[error("LegacyVoteAccount")]
    LegacyVoteAccount,
    /// Failed to parse vote account.
    #[error("UnparseableVoteAccount")]
    UnparseableVoteAccount,
    /// Incorrect number of lamports provided for rent-exemption when
    /// initializing.
    #[error("WrongRentAmount")]
    WrongRentAmount,
    /// Attempted to deposit from or withdraw to pool stake account.
    #[error("InvalidPoolStakeAccountUsage")]
    InvalidPoolStakeAccountUsage,
    /// Attempted to initialize a pool that is already initialized.
    #[error("PoolAlreadyInitialized")]
    PoolAlreadyInitialized,

    // 20
    /// Provided pool on-ramp account does not match address derived from the pool
    /// account.
    #[error("InvalidPoolOnRampAccount")]
    InvalidPoolOnRampAccount,
    /// The on-ramp account for this pool does not exist; you must call `InitializePoolOnRamp`
    /// before you can perform this operation.
    #[error("OnRampDoesntExist")]
    OnRampDoesntExist,
    /// The present operation requires a `ReplenishPool` call, either because the pool stake account
    /// is in an exceptional state, or because the on-ramp account should be refreshed.
    #[error("ReplenishRequired")]
    ReplenishRequired,
    /// Withdrawal would render the pool stake account impossible to redelegate.
    /// This can only occur if the Stake Program minimum delegation increases above 1sol.
    #[error("WithdrawalViolatesPoolRequirements")]
    WithdrawalViolatesPoolRequirements,
}
impl From<SinglePoolError> for ProgramError {
    fn from(e: SinglePoolError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl ToStr for SinglePoolError {
    fn to_str(&self) -> &'static str {
        match self {
            SinglePoolError::InvalidPoolAccount =>
                "Error: Provided pool account has the wrong address for its vote account, is uninitialized, \
                     or is otherwise invalid.",
            SinglePoolError::InvalidPoolStakeAccount =>
                "Error: Provided pool stake account does not match address derived from the pool account.",
            SinglePoolError::InvalidPoolMint =>
                "Error: Provided pool mint does not match address derived from the pool account.",
            SinglePoolError::InvalidPoolStakeAuthority =>
                "Error: Provided pool stake authority does not match address derived from the pool account.",
            SinglePoolError::InvalidPoolMintAuthority =>
                "Error: Provided pool mint authority does not match address derived from the pool account.",
            SinglePoolError::InvalidPoolMplAuthority =>
                "Error: Provided pool MPL authority does not match address derived from the pool account.",
            SinglePoolError::InvalidMetadataAccount =>
                "Error: Provided metadata account does not match metadata account derived for pool mint.",
            SinglePoolError::InvalidMetadataSigner =>
                "Error: Authorized withdrawer provided for metadata update does not match the vote account.",
            SinglePoolError::DepositTooSmall =>
                "Error: Not enough lamports provided for deposit to result in one pool token.",
            SinglePoolError::WithdrawalTooSmall =>
                "Error: Not enough pool tokens provided to withdraw stake worth one lamport.",
            SinglePoolError::WithdrawalTooLarge =>
                "Error: Not enough stake to cover the provided quantity of pool tokens. \
                    This typically means the value exists in the pool as activating stake, \
                    and an epoch is required for it to become available. Otherwise, it means \
                    active stake in the onramp must be moved via `ReplenishPool`.",
            SinglePoolError::SignatureMissing => "Error: Required signature is missing.",
            SinglePoolError::WrongStakeState => "Error: Stake account is not in the state expected by the program.",
            SinglePoolError::ArithmeticOverflow => "Error: Unsigned subtraction crossed the zero.",
            SinglePoolError::UnexpectedMathError =>
                "Error: A calculation failed unexpectedly. \
                     (This error should never be surfaced; it stands in for failure conditions that should never be reached.)",
            SinglePoolError::UnparseableVoteAccount => "Error: Failed to parse vote account.",
            SinglePoolError::LegacyVoteAccount =>
                "Error: The V0_23_5 vote account type is unsupported and should be upgraded via `convert_to_current()`.",
            SinglePoolError::WrongRentAmount =>
                "Error: Incorrect number of lamports provided for rent-exemption when initializing.",
            SinglePoolError::InvalidPoolStakeAccountUsage =>
                "Error: Attempted to deposit from or withdraw to pool stake account.",
            SinglePoolError::PoolAlreadyInitialized =>
                "Error: Attempted to initialize a pool that is already initialized.",
            SinglePoolError::InvalidPoolOnRampAccount =>
                "Error: Provided pool onramp account does not match address derived from the pool account.",
            SinglePoolError::OnRampDoesntExist =>
                "Error: The onramp account for this pool does not exist; you must call `InitializePoolOnRamp` \
                     before you can perform this operation.",
            SinglePoolError::ReplenishRequired =>
                "Error: The present operation requires a `ReplenishPool` call, either because the pool stake account \
                    is in an exceptional state, or because the on-ramp account should be refreshed.",
            SinglePoolError::WithdrawalViolatesPoolRequirements =>
                "Error: Withdrawal would render the pool stake account impossible to redelegate. \
                    This can only occur if the Stake Program minimum delegation increases above 1 sol.",
        }
    }
}
