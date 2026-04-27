use {
    crate::config::*,
    solana_clock::Epoch,
    solana_instruction::Instruction,
    solana_native_token::LAMPORTS_PER_SOL,
    solana_program_pack::Pack,
    solana_pubkey::Pubkey,
    solana_rent::Rent,
    solana_stake_interface::{
        self as stake,
        state::{Meta, Stake, StakeStateV2},
    },
    solana_system_interface::instruction as system_instruction,
    solana_sysvar as sysvar,
    spl_token_interface::{
        self as spl_token,
        state::{Account as TokenAccount, Mint},
    },
};

pub const PHANTOM_TOKENS: u64 = LAMPORTS_PER_SOL;

pub async fn get_rent(config: &Config) -> Result<Rent, Error> {
    let rent_data = config
        .get_initialized_account(sysvar::rent::id())
        .await?
        .unwrap();
    let rent = bincode::deserialize::<Rent>(&rent_data.data)?;

    Ok(rent)
}

pub async fn get_minimum_pool_balance(config: &Config) -> Result<u64, Error> {
    Ok(std::cmp::max(
        config.rpc_client.get_stake_minimum_delegation().await?,
        LAMPORTS_PER_SOL,
    ))
}

pub async fn get_stake_info(
    config: &Config,
    stake_account_address: Pubkey,
) -> Result<Option<(Meta, Stake)>, Error> {
    if let Some(account) = config
        .get_initialized_account(stake_account_address)
        .await?
    {
        match bincode::deserialize::<StakeStateV2>(&account.data)? {
            StakeStateV2::Stake(meta, stake, _) => Ok(Some((meta, stake))),
            StakeStateV2::Initialized(_) => {
                Err(format!("Stake account {} is undelegated", stake_account_address).into())
            }
            StakeStateV2::Uninitialized => {
                Err(format!("Stake account {} is uninitialized", stake_account_address).into())
            }
            StakeStateV2::RewardsPool => unimplemented!(),
        }
    } else {
        Ok(None)
    }
}

pub async fn get_token_info(
    config: &Config,
    token_account_address: Pubkey,
    mint_address: Pubkey,
) -> Result<Option<TokenAccount>, Error> {
    if let Some(account) = config
        .get_initialized_account(token_account_address)
        .await?
    {
        match TokenAccount::unpack(&account.data) {
            Ok(token_account)
                if account.owner == spl_token::id() && token_account.mint == mint_address =>
            {
                Ok(Some(token_account))
            }
            _ => Err(format!("Invalid token account {}", token_account_address).into()),
        }
    } else {
        Ok(None)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct StakeSummary {
    // `stake.delegation.stake` if activating or effective, but not if inactive
    pub stake: u64,
    // all non-rent lamports, including stake
    pub usable_lamports: u64,
    // initialized, deactivating or deactivated
    pub dedelegated: bool,
    // self-explanatory
    pub exists: bool,
}

impl StakeSummary {
    pub fn nav(self, other: Self) -> u64 {
        self.usable_lamports.saturating_add(other.usable_lamports)
    }

    pub fn excess_lamports(self, other: Self) -> u64 {
        self.usable_lamports
            .saturating_add(other.usable_lamports)
            .saturating_sub(self.stake)
            .saturating_sub(other.stake)
    }
}

pub async fn get_stake_summaries(
    config: &Config,
    stake_account_addresses: &[Pubkey],
    rent_exempt_reserve: u64,
    current_epoch: Epoch,
) -> Result<Vec<StakeSummary>, Error> {
    let stake_accounts = config
        .rpc_client
        .get_multiple_accounts(stake_account_addresses)
        .await?;

    let mut summaries = vec![];
    for stake_account in &stake_accounts {
        let summary = match stake_account {
            Some(account) if !account.data.is_empty() => {
                // if this assert ever triggers, multistake or another account change has landed.
                // this function should be updated to use real stake account sizes.
                // we may be fetching hundreds of accounts here so memoize the rents
                assert_eq!(
                    account.data.len(),
                    StakeStateV2::size_of(),
                    "StakeStateV2 is no longer canonical, or StakeStateV2::size_of() is no longer 200."
                );

                match bincode::deserialize::<StakeStateV2>(&account.data) {
                    // typical stake state. either activating or effective can be "pool stake"
                    Ok(StakeStateV2::Stake(_, Stake { delegation, .. }, _)) => {
                        let stake = if delegation.activation_epoch <= current_epoch
                            && delegation.deactivation_epoch >= current_epoch
                        {
                            delegation.stake
                        } else {
                            0
                        };

                        StakeSummary {
                            stake,
                            usable_lamports: account.lamports.saturating_sub(rent_exempt_reserve),
                            dedelegated: delegation.deactivation_epoch != u64::MAX,
                            exists: true,
                        }
                    }
                    // impossible for main stake, routine for onramp
                    Ok(StakeStateV2::Initialized(_)) => StakeSummary {
                        stake: 0,
                        usable_lamports: account.lamports.saturating_sub(rent_exempt_reserve),
                        dedelegated: true,
                        exists: true,
                    },
                    _ => unreachable!(),
                }
            }
            // impossible for main stake, possible if onramp never created.
            // we ignore lamports in an uninitialized onramp since we will never use them for math
            _ => StakeSummary {
                stake: 0,
                usable_lamports: 0,
                dedelegated: true,
                exists: false,
            },
        };

        summaries.push(summary);
    }

    Ok(summaries)
}

pub async fn get_token_supplies(
    config: &Config,
    mint_addresses: &[Pubkey],
) -> Result<Vec<u64>, Error> {
    let mint_accounts = config
        .rpc_client
        .get_multiple_accounts(mint_addresses)
        .await?;

    let mut supplies = vec![];
    for mint_account in &mint_accounts {
        let supply = if let Some(account) = mint_account {
            match Mint::unpack(&account.data) {
                Ok(mint) => mint.supply,
                _ => 0,
            }
        } else {
            0
        };

        supplies.push(supply.saturating_add(PHANTOM_TOKENS));
    }

    Ok(supplies)
}

pub async fn create_uninitialized_stake_account_instruction(
    config: &Config,
    payer: &Pubkey,
    stake_account: &Pubkey,
) -> Result<Instruction, Error> {
    let rent_amount = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(StakeStateV2::size_of())
        .await?;

    Ok(system_instruction::create_account(
        payer,
        stake_account,
        rent_amount,
        StakeStateV2::size_of() as u64,
        &stake::program::id(),
    ))
}
