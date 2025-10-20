// XXX this file will be deleted and replaced with a stake program client once i
// write one

use {
    crate::config::*,
    solana_instruction::Instruction,
    solana_native_token::LAMPORTS_PER_SOL,
    solana_pubkey::Pubkey,
    solana_rent::Rent,
    solana_stake_interface::{
        self as stake,
        state::{Meta, Stake, StakeStateV2},
    },
    solana_system_interface::instruction as system_instruction,
    solana_sysvar as sysvar,
    spl_token::{solana_program::program_pack::Pack, state::Mint},
};

pub async fn get_rent(config: &Config) -> Result<Rent, Error> {
    let rent_data = config
        .program_client
        .get_account(sysvar::rent::id())
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
    stake_account_address: &Pubkey,
) -> Result<Option<(Meta, Stake)>, Error> {
    if let Some(stake_account) = config
        .program_client
        .get_account(*stake_account_address)
        .await?
    {
        match bincode::deserialize::<StakeStateV2>(&stake_account.data)? {
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

pub async fn get_available_stakes(
    config: &Config,
    stake_account_addresses: &[Pubkey],
    minimum_pool_balance: u64,
) -> Result<Vec<u64>, Error> {
    let stake_accounts = config
        .rpc_client
        .get_multiple_accounts(stake_account_addresses)
        .await?;

    let mut delegations = vec![];
    for stake_account in &stake_accounts {
        let delegation = if let Some(account) = stake_account {
            match bincode::deserialize::<StakeStateV2>(&account.data) {
                Ok(StakeStateV2::Stake(_, stake, _)) => {
                    stake.delegation.stake.saturating_sub(minimum_pool_balance)
                }
                _ => 0,
            }
        } else {
            0
        };
        delegations.push(delegation);
    }

    Ok(delegations)
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
        supplies.push(supply);
    }

    Ok(supplies)
}

pub async fn create_uninitialized_stake_account_instruction(
    config: &Config,
    payer: &Pubkey,
    stake_account: &Pubkey,
) -> Result<Instruction, Error> {
    let rent_amount = config
        .program_client
        .get_minimum_balance_for_rent_exemption(std::mem::size_of::<StakeStateV2>())
        .await?;

    Ok(system_instruction::create_account(
        payer,
        stake_account,
        rent_amount,
        std::mem::size_of::<StakeStateV2>() as u64,
        &stake::program::id(),
    ))
}
