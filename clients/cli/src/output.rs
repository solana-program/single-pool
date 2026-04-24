use {
    crate::config::Config,
    console::style,
    serde::{Deserialize, Serialize},
    serde_with::{serde_as, DisplayFromStr},
    solana_cli_output::{
        display::{build_balance_message, writeln_name_value},
        QuietDisplay, VerboseDisplay,
    },
    solana_pubkey::Pubkey,
    solana_signature::Signature,
    spl_single_pool::{
        self, find_pool_mint_address, find_pool_mint_authority_address,
        find_pool_mpl_authority_address, find_pool_onramp_address, find_pool_stake_address,
        find_pool_stake_authority_address,
    },
    std::fmt::{Display, Formatter, Result, Write},
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommandOutput<T>
where
    T: Serialize + Display + QuietDisplay + VerboseDisplay,
{
    pub(crate) command_name: String,
    pub(crate) command_output: T,
}

impl<T> Display for CommandOutput<T>
where
    T: Serialize + Display + QuietDisplay + VerboseDisplay,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.command_output, f)
    }
}

impl<T> QuietDisplay for CommandOutput<T>
where
    T: Serialize + Display + QuietDisplay + VerboseDisplay,
{
    fn write_str(&self, w: &mut dyn std::fmt::Write) -> std::fmt::Result {
        QuietDisplay::write_str(&self.command_output, w)
    }
}

impl<T> VerboseDisplay for CommandOutput<T>
where
    T: Serialize + Display + QuietDisplay + VerboseDisplay,
{
    fn write_str(&self, w: &mut dyn std::fmt::Write) -> std::fmt::Result {
        writeln_name_value(w, "Command:", &self.command_name)?;
        VerboseDisplay::write_str(&self.command_output, w)
    }
}

pub fn format_output<T>(config: &Config, command_name: String, command_output: T) -> String
where
    T: Serialize + Display + QuietDisplay + VerboseDisplay,
{
    config.output_format.formatted_string(&CommandOutput {
        command_name,
        command_output,
    })
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureOutput {
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub signature: Option<Signature>,
}

impl QuietDisplay for SignatureOutput {}
impl VerboseDisplay for SignatureOutput {}

impl Display for SignatureOutput {
    fn fmt(&self, f: &mut Formatter) -> Result {
        writeln!(f)?;

        if let Some(signature) = self.signature {
            writeln_name_value(f, "Signature:", &signature.to_string())?;
        }

        Ok(())
    }
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StakePoolOutput {
    #[serde_as(as = "DisplayFromStr")]
    pub pool_address: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub vote_account_address: Pubkey,
    pub net_asset_value: u64,
    pub undelegated_lamports: u64,
    pub token_supply: u64,
    pub main_stake_dedelegated: bool,
    pub onramp_exists: bool,
    #[serde(skip)]
    pub minimum_delegation: u64,

    #[serde_as(as = "Option<DisplayFromStr>")]
    pub signature: Option<Signature>,
}

impl QuietDisplay for StakePoolOutput {}
impl VerboseDisplay for StakePoolOutput {
    fn write_str(&self, w: &mut dyn Write) -> Result {
        writeln!(w)?;
        writeln!(w, "{}", style("SPL Single-Validator Stake Pool").bold())?;
        writeln_name_value(w, "  Pool address:", &self.pool_address.to_string())?;
        writeln_name_value(
            w,
            "  Vote account address:",
            &self.vote_account_address.to_string(),
        )?;

        writeln_name_value(
            w,
            "  Pool main stake account address:",
            &find_pool_stake_address(&spl_single_pool::id(), &self.pool_address).to_string(),
        )?;
        writeln_name_value(
            w,
            "  Pool onramp stake account address:",
            &find_pool_onramp_address(&spl_single_pool::id(), &self.pool_address).to_string(),
        )?;
        writeln_name_value(
            w,
            "  Pool mint address:",
            &find_pool_mint_address(&spl_single_pool::id(), &self.pool_address).to_string(),
        )?;
        writeln_name_value(
            w,
            "  Pool stake authority address:",
            &find_pool_stake_authority_address(&spl_single_pool::id(), &self.pool_address)
                .to_string(),
        )?;
        writeln_name_value(
            w,
            "  Pool mint authority address:",
            &find_pool_mint_authority_address(&spl_single_pool::id(), &self.pool_address)
                .to_string(),
        )?;
        writeln_name_value(
            w,
            "  Pool MPL authority address:",
            &find_pool_mpl_authority_address(&spl_single_pool::id(), &self.pool_address)
                .to_string(),
        )?;

        writeln_name_value(w, "  Net asset value:", &self.net_asset_value.to_string())?;
        writeln_name_value(
            w,
            "  Undelegated lamports:",
            &self.undelegated_lamports.to_string(),
        )?;
        writeln_name_value(
            w,
            "  Notional token supply:",
            &self.token_supply.to_string(),
        )?;

        self.print_shared_warnings(w)?;

        if let Some(signature) = self.signature {
            writeln!(w)?;
            writeln_name_value(w, "Signature:", &signature.to_string())?;
        }

        Ok(())
    }
}

impl Display for StakePoolOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f)?;
        writeln!(f, "{}", style("SPL Single-Validator Stake Pool").bold())?;
        writeln_name_value(f, "  Pool address:", &self.pool_address.to_string())?;
        writeln_name_value(
            f,
            "  Vote account address:",
            &self.vote_account_address.to_string(),
        )?;
        writeln_name_value(f, "  Net asset value:", &self.net_asset_value.to_string())?;
        writeln_name_value(
            f,
            "  Notional token supply:",
            &self.token_supply.to_string(),
        )?;

        self.print_shared_warnings(f)?;

        if let Some(signature) = self.signature {
            writeln!(f)?;
            writeln_name_value(f, "Signature:", &signature.to_string())?;
        }

        Ok(())
    }
}

impl StakePoolOutput {
    fn print_shared_warnings(&self, w: &mut dyn Write) -> Result {
        // these are not mutually exclusive, we just use `else if` for ux reasons.
        // namely, dont tell the user to create an onramp if the pool is unusable,
        // and dont tell the user to replenish if theres no onramp yet.
        // it is a bit weird tho because they cant fix an undelegated main account without an onramp.
        // but we *dont* want to tell an unsavvy user to blindly replenish in this case,
        // since *we* dont know if the vote account is back in good standing without a bunch of out-of-scope nonsense
        if self.main_stake_dedelegated {
            writeln!(
                w,
                "{} This validator's vote account may be delinquent or closed!",
                style("/!\\ POOL STAKE IS UNDELEGATED /!\\").bold(),
            )?;
        } else if !self.onramp_exists {
            writeln!(
                w,
                "{} Onramp does not exist; use `spl-single-pool manage create-on-ramp` to create it",
                style("/!\\").bold(),
            )?;
        } else if self.undelegated_lamports >= self.minimum_delegation {
            writeln!(
                w,
                "{} This pool has {} not earning rewards; use `spl-single-pool manage replenish-pool` to delegate it",
                style("/!\\").bold(),
                build_balance_message(self.undelegated_lamports, false, true),
            )?;
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StakePoolListOutput(pub Vec<StakePoolOutput>);

impl QuietDisplay for StakePoolListOutput {}
impl VerboseDisplay for StakePoolListOutput {
    fn write_str(&self, w: &mut dyn Write) -> Result {
        let mut nav = 0;
        for svsp in &self.0 {
            VerboseDisplay::write_str(svsp, w)?;
            nav += svsp.net_asset_value;
        }

        writeln_name_value(
            w,
            "\nTotal value:",
            &build_balance_message(nav, false, true),
        )?;

        Ok(())
    }
}

impl Display for StakePoolListOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let mut nav = 0;
        for svsp in &self.0 {
            svsp.fmt(f)?;
            nav += svsp.net_asset_value;
        }

        writeln_name_value(
            f,
            "\nTotal value:",
            &build_balance_message(nav, false, true),
        )?;

        Ok(())
    }
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepositOutput {
    #[serde_as(as = "DisplayFromStr")]
    pub pool_address: Pubkey,
    pub token_amount: Option<u64>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub signature: Option<Signature>,
}

impl QuietDisplay for DepositOutput {}
impl VerboseDisplay for DepositOutput {}

impl Display for DepositOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f)?;
        writeln_name_value(f, "Pool address:", &self.pool_address.to_string())?;

        let token_amount = if let Some(amount) = self.token_amount {
            &amount.to_string()
        } else {
            "(cannot display in simulation)"
        };
        writeln_name_value(f, "Token amount:", token_amount)?;

        if let Some(signature) = self.signature {
            writeln!(f)?;
            writeln_name_value(f, "Signature:", &signature.to_string())?;
        }

        Ok(())
    }
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawOutput {
    #[serde_as(as = "DisplayFromStr")]
    pub pool_address: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub stake_account_address: Pubkey,
    pub stake_amount: Option<u64>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub signature: Option<Signature>,
}

impl QuietDisplay for WithdrawOutput {}
impl VerboseDisplay for WithdrawOutput {}

impl Display for WithdrawOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f)?;
        writeln_name_value(f, "Pool address:", &self.pool_address.to_string())?;
        writeln_name_value(
            f,
            "Stake account address:",
            &self.stake_account_address.to_string(),
        )?;

        let stake_amount = if let Some(amount) = self.stake_amount {
            &amount.to_string()
        } else {
            "(cannot display in simulation)"
        };
        writeln_name_value(f, "Stake amount:", stake_amount)?;

        if let Some(signature) = self.signature {
            writeln!(f)?;
            writeln_name_value(f, "Signature:", &signature.to_string())?;
        }

        Ok(())
    }
}
