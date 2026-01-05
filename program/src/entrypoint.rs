//! Program entrypoint

#![cfg(all(target_os = "solana", not(feature = "no-entrypoint")))]

use {
    crate::{error::SinglePoolError, processor::Processor},
    solana_account_info::AccountInfo,
    solana_msg::msg,
    solana_program_entrypoint::{entrypoint, ProgramResult},
    solana_pubkey::Pubkey,
    solana_security_txt::security_txt,
};

entrypoint!(process_instruction);
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if let Err(error) = Processor::process(program_id, accounts, instruction_data) {
        // catch the error so we can print it
        msg!(error.to_str::<SinglePoolError>());
        Err(error)
    } else {
        Ok(())
    }
}

security_txt! {
    // Required fields
    name: "SPL Single-Validator Stake Pool",
    project_url: "https://spl.solana.com/single-pool",
    contacts: "link:https://github.com/solana-labs/solana-program-library/security/advisories/new,mailto:security@anza.xyz,discord:https://discord.gg/solana",
    policy: "https://github.com/solana-labs/solana-program-library/blob/master/SECURITY.md",

    // Optional Fields
    preferred_languages: "en",
    source_code: "https://github.com/solana-program/single-pool/tree/main/program",
    auditors: "https://spl.solana.com/single-pool#security-audits"
}
