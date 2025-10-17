use {
    clap::{
        builder::{PossibleValuesParser, TypedValueParser},
        ArgGroup, ArgMatches, Args, Parser, Subcommand,
    },
    solana_clap_v3_utils::{
        input_parsers::{
            parse_url_or_moniker,
            signer::{SignerSource, SignerSourceParserBuilder},
            Amount,
        },
        keypair::pubkey_from_path,
    },
    solana_cli_output::OutputFormat,
    solana_pubkey::Pubkey,
    spl_single_pool::{self, find_pool_address},
};

#[derive(Clone, Debug, Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    /// Configuration file to use
    #[clap(global(true), short = 'C', long = "config", id = "PATH")]
    pub config_file: Option<String>,

    /// Show additional information
    #[clap(global(true), short, long)]
    pub verbose: bool,

    /// Simulate transaction instead of executing
    #[clap(global(true), long, alias = "dryrun")]
    pub dry_run: bool,

    /// URL for Solana JSON RPC or moniker (or their first letter):
    /// [mainnet-beta, testnet, devnet, localhost].
    /// Default from the configuration file.
    #[clap(
        global(true),
        short = 'u',
        long = "url",
        id = "URL_OR_MONIKER",
        value_parser = parse_url_or_moniker,
    )]
    pub json_rpc_url: Option<String>,

    /// Specify the fee-payer account. This may be a keypair file, the ASK
    /// keyword or the pubkey of an offline signer, provided an appropriate
    /// --signer argument is also passed. Defaults to the client keypair.
    #[clap(
        global(true),
        long,
        id = "PAYER_KEYPAIR",
        value_parser = SignerSourceParserBuilder::default().allow_all().build(),
    )]
    pub fee_payer: Option<SignerSource>,

    /// Return information in specified output format
    #[clap(
        global(true),
        long = "output",
        id = "FORMAT",
        conflicts_with = "verbose",
        value_parser = PossibleValuesParser::new(["json", "json-compact"]).map(|o| parse_output_format(&o)),
    )]
    pub output_format: Option<OutputFormat>,

    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Clone, Debug, Subcommand)]
pub enum Command {
    /// Commands used to initialize or manage existing single-validator stake
    /// pools. Other than initializing new pools, most users should never
    /// need to use these.
    Manage(ManageCli),

    /// Deposit delegated stake into a pool in exchange for pool tokens, closing
    /// out the original stake account. Provide either a stake account
    /// address, or a pool or vote account address along with the
    /// --default-stake-account flag to use an account created with
    /// create-stake.
    Deposit(DepositCli),

    /// Withdraw stake into a new stake account, burning tokens in exchange.
    /// Provide either pool or vote account address, plus either an amount of
    /// tokens to burn or the ALL keyword to burn all.
    Withdraw(WithdrawCli),

    /// WARNING: This command is DEPRECATED and will be removed in a future release.
    /// Create and delegate a new stake account to a given validator, using a
    /// default address linked to the intended depository pool
    CreateDefaultStake(CreateStakeCli),

    /// Display info for one or all single-validator stake pool(s)
    Display(DisplayCli),
}

#[derive(Clone, Debug, Parser)]
pub struct ManageCli {
    #[clap(subcommand)]
    pub manage: ManageCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum ManageCommand {
    /// Permissionlessly create the single-validator stake pool for a given
    /// validator vote account if one does not already exist. The fee payer
    /// also pays rent-exemption for accounts, along with the
    /// cluster-configured minimum stake delegation
    Initialize(InitializeCli),

    /// Permissionlessly re-stake the main pool stake account if it was
    /// deactivated from a delinquent validator, move active stake from the
    /// on-ramp account into the main account, and move and delegate excess
    /// lamports from the main account in the on-ramp account.
    ReplenishPool(ReplenishCli),

    /// Permissionlessly create default MPL token metadata for the pool mint.
    /// Normally this is done automatically upon initialization, so this
    /// does not need to be called.
    CreateTokenMetadata(CreateMetadataCli),

    /// Modify the MPL token metadata associated with the pool mint. This action
    /// can only be performed by the validator vote account's withdraw
    /// authority
    UpdateTokenMetadata(UpdateMetadataCli),

    /// Permissionlessly create the on-ramp account for an existing single-
    /// validator stake pool, necessary for calling `ReplenishPool`.
    /// This does NOT need to be called after `Initialize`: initialization
    /// takes care of this in `>=v2.0.0`. Only existing pools created by
    /// `1.0.x` need to to create the on-ramp explicitly.
    CreateOnRamp(CreateOnRampCli),
}

#[derive(Clone, Debug, Args)]
pub struct InitializeCli {
    /// The vote account to create the pool for
    #[clap(value_parser = |p: &str| parse_address(p, "vote_account_address"))]
    pub vote_account_address: Pubkey,

    /// Do not create MPL metadata for the pool mint
    #[clap(long)]
    pub skip_metadata: bool,
}

#[derive(Clone, Debug, Args)]
#[clap(group(pool_source_group()))]
pub struct ReplenishCli {
    /// The pool to replenish
    #[clap(short, long = "pool", value_parser = |p: &str| parse_address(p, "pool_address"))]
    pub pool_address: Option<Pubkey>,

    /// The vote account corresponding to the pool to replenish
    #[clap(long = "vote-account", value_parser = |p: &str| parse_address(p, "vote_account_address"))]
    pub vote_account_address: Option<Pubkey>,
}

#[derive(Clone, Debug, Args)]
#[clap(group(ArgGroup::new("stake-source").required(true).args(&["stake-account-address", "default-stake-account"])))]
#[clap(group(pool_source_group().required(false)))]
pub struct DepositCli {
    /// The stake account to deposit from. Must be in the same activation state
    /// as the pool's stake account
    #[clap(value_parser = |p: &str| parse_address(p, "stake_account_address"))]
    pub stake_account_address: Option<Pubkey>,

    /// WARNING: This flag is DEPRECATED and will be removed in a future release.
    /// Instead of using a stake account by address, use the user's default
    /// account for a specified pool
    #[clap(
        short,
        long,
        conflicts_with = "stake-account-address",
        requires = "pool-source"
    )]
    pub default_stake_account: bool,

    /// The pool to deposit into. Optional when stake account is provided
    #[clap(short, long = "pool", value_parser = |p: &str| parse_address(p, "pool_address"))]
    pub pool_address: Option<Pubkey>,

    /// The vote account corresponding to the pool to deposit into. Optional
    /// when stake account or pool is provided
    #[clap(long = "vote-account", value_parser = |p: &str| parse_address(p, "vote_account_address"))]
    pub vote_account_address: Option<Pubkey>,

    /// Signing authority on the stake account to be deposited. Defaults to the
    /// client keypair
    #[clap(long = "withdraw-authority", id = "STAKE_WITHDRAW_AUTHORITY_KEYPAIR", value_parser = SignerSourceParserBuilder::default().allow_all().build(),)]
    pub stake_withdraw_authority: Option<SignerSource>,

    /// The token account to mint to. Defaults to the client keypair's
    /// associated token account
    #[clap(long = "token-account", value_parser = |p: &str| parse_address(p, "token_account_address"))]
    pub token_account_address: Option<Pubkey>,

    /// The wallet to refund stake account rent to. Defaults to the client
    /// keypair's pubkey
    #[clap(long = "recipient", value_parser = |p: &str| parse_address(p, "lamport_recipient_address"))]
    pub lamport_recipient_address: Option<Pubkey>,
}

#[derive(Clone, Debug, Args)]
#[clap(group(pool_source_group()))]
pub struct WithdrawCli {
    /// Amount of tokens to burn for withdrawal
    #[clap(value_parser = Amount::parse_decimal_or_all)]
    pub token_amount: Amount,

    /// The token account to withdraw from. Defaults to the associated token
    /// account for the pool mint
    #[clap(long = "token-account", value_parser = |p: &str| parse_address(p, "token_account_address"))]
    pub token_account_address: Option<Pubkey>,

    /// The pool to withdraw from
    #[clap(short, long = "pool", value_parser = |p: &str| parse_address(p, "pool_address"))]
    pub pool_address: Option<Pubkey>,

    /// The vote account corresponding to the pool to withdraw from
    #[clap(long = "vote-account", value_parser = |p: &str| parse_address(p, "vote_account_address"))]
    pub vote_account_address: Option<Pubkey>,

    /// Signing authority on the token account. Defaults to the client keypair
    #[clap(long = "token-authority", id = "TOKEN_AUTHORITY_KEYPAIR", value_parser = SignerSourceParserBuilder::default().allow_all().build())]
    pub token_authority: Option<SignerSource>,

    /// Authority to assign to the new stake account. Defaults to the pubkey of
    /// the client keypair
    #[clap(long = "stake-authority", value_parser = |p: &str| parse_address(p, "stake_authority_address"))]
    pub stake_authority_address: Option<Pubkey>,

    /// Deactivate stake account after withdrawal
    #[clap(long)]
    pub deactivate: bool,
}

#[derive(Clone, Debug, Args)]
#[clap(group(pool_source_group()))]
pub struct CreateMetadataCli {
    /// The pool to create default MPL token metadata for
    #[clap(short, long = "pool", value_parser = |p: &str| parse_address(p, "pool_address"))]
    pub pool_address: Option<Pubkey>,

    /// The vote account corresponding to the pool to create metadata for
    #[clap(long = "vote-account", value_parser = |p: &str| parse_address(p, "vote_account_address"))]
    pub vote_account_address: Option<Pubkey>,
}

#[derive(Clone, Debug, Args)]
#[clap(group(pool_source_group()))]
pub struct UpdateMetadataCli {
    /// New name for the pool token
    #[clap(validator = is_valid_token_name)]
    pub token_name: String,

    /// New ticker symbol for the pool token
    #[clap(validator = is_valid_token_symbol)]
    pub token_symbol: String,

    /// Optional external URI for the pool token. Leaving this argument blank
    /// will clear any existing value
    #[clap(validator = is_valid_token_uri)]
    pub token_uri: Option<String>,

    /// The pool to change MPL token metadata for
    #[clap(short, long = "pool", value_parser = |p: &str| parse_address(p, "pool_address"))]
    pub pool_address: Option<Pubkey>,

    /// The vote account corresponding to the pool to create metadata for
    #[clap(long = "vote-account", value_parser = |p: &str| parse_address(p, "vote_account_address"))]
    pub vote_account_address: Option<Pubkey>,

    /// Authorized withdrawer for the vote account, to prove validator
    /// ownership. Defaults to the client keypair
    #[clap(long, id = "AUTHORIZED_WITHDRAWER_KEYPAIR", value_parser = SignerSourceParserBuilder::default().allow_all().build())]
    pub authorized_withdrawer: Option<SignerSource>,
}

#[derive(Clone, Debug, Args)]
#[clap(group(pool_source_group()))]
pub struct CreateStakeCli {
    /// Number of lamports to stake
    pub lamports: u64,

    /// The pool to create a stake account for
    #[clap(short, long = "pool", value_parser = |p: &str| parse_address(p, "pool_address"))]
    pub pool_address: Option<Pubkey>,

    /// The vote account corresponding to the pool to create stake for
    #[clap(long = "vote-account", value_parser = |p: &str| parse_address(p, "vote_account_address"))]
    pub vote_account_address: Option<Pubkey>,

    /// Authority to assign to the new stake account. Defaults to the pubkey of
    /// the client keypair
    #[clap(long = "stake-authority", value_parser = |p: &str| parse_address(p, "stake_authority_address"))]
    pub stake_authority_address: Option<Pubkey>,
}

#[derive(Clone, Debug, Args)]
#[clap(group(pool_source_group().arg("all")))]
pub struct DisplayCli {
    /// The pool to display
    #[clap(value_parser = |p: &str| parse_address(p, "pool_address"))]
    pub pool_address: Option<Pubkey>,

    /// The vote account corresponding to the pool to display
    #[clap(long = "vote-account", value_parser = |p: &str| parse_address(p, "vote_account_address"))]
    pub vote_account_address: Option<Pubkey>,

    /// Display all pools
    #[clap(long)]
    pub all: bool,
}

#[derive(Clone, Debug, Args)]
#[clap(group(pool_source_group()))]
pub struct CreateOnRampCli {
    /// The pool to create the on-ramp stake account for
    #[clap(short, long = "pool", value_parser = |p: &str| parse_address(p, "pool_address"))]
    pub pool_address: Option<Pubkey>,

    /// The vote account corresponding to the pool to create the on-ramp for
    #[clap(long = "vote-account", value_parser = |p: &str| parse_address(p, "vote_account_address"))]
    pub vote_account_address: Option<Pubkey>,
}

fn pool_source_group() -> ArgGroup<'static> {
    ArgGroup::new("pool-source")
        .required(true)
        .args(&["pool-address", "vote-account-address"])
}

fn parse_address(path: &str, name: &str) -> Result<Pubkey, String> {
    let mut wallet_manager = None;
    pubkey_from_path(&ArgMatches::default(), path, name, &mut wallet_manager)
        .map_err(|_| format!("Failed to load pubkey {} at {}", name, path))
}

pub fn parse_output_format(output_format: &str) -> OutputFormat {
    match output_format {
        "json" => OutputFormat::Json,
        "json-compact" => OutputFormat::JsonCompact,
        _ => unreachable!(),
    }
}

pub fn is_valid_token_name(s: &str) -> Result<(), String> {
    if s.len() > 32 {
        Err("Maximum token name length is 32 characters".to_string())
    } else {
        Ok(())
    }
}

pub fn is_valid_token_symbol(s: &str) -> Result<(), String> {
    if s.len() > 10 {
        Err("Maximum token symbol length is 10 characters".to_string())
    } else {
        Ok(())
    }
}

pub fn is_valid_token_uri(s: &str) -> Result<(), String> {
    if s.len() > 200 {
        Err("Maximum token URI length is 200 characters".to_string())
    } else {
        Ok(())
    }
}

pub fn pool_address_from_args(maybe_pool: Option<Pubkey>, maybe_vote: Option<Pubkey>) -> Pubkey {
    if let Some(pool_address) = maybe_pool {
        pool_address
    } else if let Some(vote_account_address) = maybe_vote {
        find_pool_address(&spl_single_pool::id(), &vote_account_address)
    } else {
        unreachable!()
    }
}
