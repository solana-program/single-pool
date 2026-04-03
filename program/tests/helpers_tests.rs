mod helpers;

use {
    helpers::*, solana_native_token::LAMPORTS_PER_SOL, solana_program_test::*, std::path::Path,
    strum::IntoEnumIterator, test_case::test_matrix,
};

// sanity: version -> file mappings resolve
#[test]
fn test_program_versions() {
    for version in StakeProgramVersion::iter() {
        let Some(basename) = version.basename() else {
            return;
        };

        let path = Path::new("tests/fixtures").join(format!("{basename}.so"));
        assert!(path.exists());
    }
}

// sanity: there is always a Stable program. otherwise, we might "pass" by skipping all tests
#[test]
fn test_live_program() {
    assert!(StakeProgramVersion::Stable.basename().is_some());
}

// temporary sanity to guard against ProgramTest breakage
// this is designed to break on purpose if we havent removed it before stake v6
#[test_matrix(
    [StakeProgramVersion::Stable, StakeProgramVersion::Beta, StakeProgramVersion::Edge]
)]
#[tokio::test]
async fn test_expected_program(stake_version: StakeProgramVersion) {
    let Some(program_test) = program_test(stake_version) else {
        return;
    };
    let mut context = program_test.start_with_context().await;

    let minimum_delegation = get_minimum_delegation(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
    )
    .await;

    if stake_version.basename().unwrap() == "solana_stake_program-v5.0.0" {
        assert_eq!(minimum_delegation, LAMPORTS_PER_SOL);
    } else {
        assert_eq!(minimum_delegation, 1);
    }
}
