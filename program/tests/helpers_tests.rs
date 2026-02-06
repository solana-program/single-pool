mod helpers;

use {helpers::*, std::path::Path, strum::IntoEnumIterator};

// sanity: version -> file mappings resolve
#[test]
fn test_program_versions() {
    for version in StakeProgramVersion::iter() {
        let Some(basename) = version.basename() else {
            return;
        };

        let path = Path::new("tests/fixtures").join(format!("{}.so", basename));
        assert!(path.exists());
    }
}

// sanity: there always a Live program. otherwise, we might "pass" by skipping all tests
#[test]
fn test_live_program() {
    assert!(StakeProgramVersion::Live.basename().is_some());
}
