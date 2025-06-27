//! Test loading of various workspace members, including duplicate members

prelude!();

use std::path::PathBuf;

use crate::workspace::{
    Diagnostics, FileSourceLoader, FoundKind, Manifest, ManifestLoader, WorkspaceFilter,
    MANIFEST_FILE,
};

fn load_manifest() -> Manifest {
    let manifest_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/tests/workspace")
        .join(MANIFEST_FILE);

    let source = Source::from_path(&manifest_file).unwrap();
    let mut sources = Sources::new();
    let id = sources.insert(source).unwrap();

    let mut diagnostics = Diagnostics::default();
    let mut source_loader = FileSourceLoader::new();
    let mut manifest = Manifest::default();

    let mut loader = ManifestLoader::new(
        id,
        &mut sources,
        &mut diagnostics,
        &mut source_loader,
        &mut manifest,
    );
    loader.load_manifest().unwrap();

    manifest
}

#[test]
pub fn manifest_binaries() {
    const EXPECTED: &[(FoundKind, &str, &str)] = &[
        (FoundKind::Binary, "a", "a"),
        (FoundKind::Binary, "b", "b"),
        (FoundKind::Binary, "multi-file-executable", "a"),
        (FoundKind::Binary, "multi-file-executable", "b"),
        (FoundKind::Binary, "named-executable", "a"),
        (FoundKind::Binary, "named-executable", "b"),
    ];

    let manifest = load_manifest();
    let found_packages = manifest
        .find_by_kind(WorkspaceFilter::All, FoundKind::Binary)
        .unwrap();
    let mut found_packages = found_packages
        .iter()
        .map(|found_package| {
            (
                found_package.found.kind,
                found_package.found.name.as_str(),
                found_package.package.name.as_str(),
            )
        })
        .collect::<Vec<_>>();

    found_packages.sort_unstable();

    assert_eq!(found_packages.as_slice(), EXPECTED);
}

#[test]
pub fn manifest_filtered_binaries() {
    const EXPECTED: &[(FoundKind, &str, &str)] = &[
        (FoundKind::Binary, "named-executable", "a"),
        (FoundKind::Binary, "named-executable", "b"),
    ];

    let manifest = load_manifest();
    let found_packages = manifest
        .find_by_kind(WorkspaceFilter::Name("named-executable"), FoundKind::Binary)
        .unwrap();
    let mut found_packages = found_packages
        .iter()
        .map(|found_package| {
            (
                found_package.found.kind,
                found_package.found.name.as_str(),
                found_package.package.name.as_str(),
            )
        })
        .collect::<Vec<_>>();

    found_packages.sort_unstable();

    assert_eq!(found_packages.as_slice(), EXPECTED);
}

#[test]
pub fn manifest_libraries() {
    const EXPECTED: &[(FoundKind, &str, &str)] = &[
        (FoundKind::Library, "a", "a"),
        (FoundKind::Library, "b", "b"),
    ];

    let manifest = load_manifest();
    let found_packages = manifest
        .find_by_kind(WorkspaceFilter::All, FoundKind::Library)
        .unwrap();
    let mut found_packages = found_packages
        .iter()
        .map(|found_package| {
            (
                found_package.found.kind,
                found_package.found.name.as_str(),
                found_package.package.name.as_str(),
            )
        })
        .collect::<Vec<_>>();

    found_packages.sort_unstable();

    assert_eq!(found_packages.as_slice(), EXPECTED);
}

#[test]
pub fn manifest_tests() {
    const EXPECTED: &[(FoundKind, &str, &str)] = &[
        (FoundKind::Test, "fire", "a"),
        (FoundKind::Test, "fire", "b"),
        (FoundKind::Test, "smoke", "a"),
        (FoundKind::Test, "smoke", "b"),
    ];

    let manifest = load_manifest();
    let found_packages = manifest
        .find_by_kind(WorkspaceFilter::All, FoundKind::Test)
        .unwrap();
    let mut found_packages = found_packages
        .iter()
        .map(|found_package| {
            (
                found_package.found.kind,
                found_package.found.name.as_str(),
                found_package.package.name.as_str(),
            )
        })
        .collect::<Vec<_>>();

    found_packages.sort_unstable();

    assert_eq!(found_packages.as_slice(), EXPECTED);
}

#[test]
pub fn manifest_examples() {
    const EXPECTED: &[(FoundKind, &str, &str)] = &[
        (FoundKind::Example, "multi-file-example", "a"),
        (FoundKind::Example, "multi-file-example", "b"),
        (FoundKind::Example, "simple", "a"),
        (FoundKind::Example, "simple", "b"),
    ];

    let manifest = load_manifest();
    let found_packages = manifest
        .find_by_kind(WorkspaceFilter::All, FoundKind::Example)
        .unwrap();
    let mut found_packages = found_packages
        .iter()
        .map(|found_package| {
            (
                found_package.found.kind,
                found_package.found.name.as_str(),
                found_package.package.name.as_str(),
            )
        })
        .collect::<Vec<_>>();

    found_packages.sort_unstable();

    assert_eq!(found_packages.as_slice(), EXPECTED);
}

#[test]
pub fn manifest_benches() {
    const EXPECTED: &[(FoundKind, &str, &str)] = &[
        (FoundKind::Bench, "collatz", "a"),
        (FoundKind::Bench, "collatz", "b"),
        (FoundKind::Bench, "multi-file-bench", "a"),
        (FoundKind::Bench, "multi-file-bench", "b"),
    ];

    let manifest = load_manifest();
    let found_packages = manifest
        .find_by_kind(WorkspaceFilter::All, FoundKind::Bench)
        .unwrap();
    let mut found_packages = found_packages
        .iter()
        .map(|found_package| {
            (
                found_package.found.kind,
                found_package.found.name.as_str(),
                found_package.package.name.as_str(),
            )
        })
        .collect::<Vec<_>>();

    found_packages.sort_unstable();

    assert_eq!(found_packages.as_slice(), EXPECTED);
}

#[test]
pub fn manifest_all() {
    const EXPECTED: &[(FoundKind, &str, &str)] = &[
        (FoundKind::Binary, "a", "a"),
        (FoundKind::Binary, "b", "b"),
        (FoundKind::Binary, "multi-file-executable", "a"),
        (FoundKind::Binary, "multi-file-executable", "b"),
        (FoundKind::Binary, "named-executable", "a"),
        (FoundKind::Binary, "named-executable", "b"),
        (FoundKind::Library, "a", "a"),
        (FoundKind::Library, "b", "b"),
        (FoundKind::Test, "fire", "a"),
        (FoundKind::Test, "fire", "b"),
        (FoundKind::Test, "smoke", "a"),
        (FoundKind::Test, "smoke", "b"),
        (FoundKind::Example, "multi-file-example", "a"),
        (FoundKind::Example, "multi-file-example", "b"),
        (FoundKind::Example, "simple", "a"),
        (FoundKind::Example, "simple", "b"),
        (FoundKind::Bench, "collatz", "a"),
        (FoundKind::Bench, "collatz", "b"),
        (FoundKind::Bench, "multi-file-bench", "a"),
        (FoundKind::Bench, "multi-file-bench", "b"),
    ];

    let manifest = load_manifest();
    let found_packages = manifest.find_all(WorkspaceFilter::All).unwrap();
    let mut found_packages = found_packages
        .iter()
        .map(|found_package| {
            (
                found_package.found.kind,
                found_package.found.name.as_str(),
                found_package.package.name.as_str(),
            )
        })
        .collect::<Vec<_>>();

    found_packages.sort_unstable();

    assert_eq!(found_packages.as_slice(), EXPECTED);
}

#[test]
pub fn manifest_all_filtered() {
    const EXPECTED: &[(FoundKind, &str, &str)] = &[
        (FoundKind::Binary, "a", "a"),
        (FoundKind::Library, "a", "a"),
    ];

    let manifest = load_manifest();
    let found_packages = manifest.find_all(WorkspaceFilter::Name("a")).unwrap();
    let mut found_packages = found_packages
        .iter()
        .map(|found_package| {
            (
                found_package.found.kind,
                found_package.found.name.as_str(),
                found_package.package.name.as_str(),
            )
        })
        .collect::<Vec<_>>();

    found_packages.sort_unstable();

    assert_eq!(found_packages.as_slice(), EXPECTED);
}
