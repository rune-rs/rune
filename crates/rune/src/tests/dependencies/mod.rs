//! `[dependencies]` in `Rune.toml`: path dependencies whose declaration
//! sources (`*.d.rn`) are compiled beside the depending package's entry points.

prelude!();

use std::path::{Path, PathBuf};

use crate::diagnostics::{Diagnostic, FatalDiagnosticKind};
use crate::workspace::{self, FileSourceLoader, Manifest, ManifestLoader, MANIFEST_FILE};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/tests/dependencies")
        .join(name)
}

/// Load the manifest of the fixture package `name`, along with the messages of
/// whatever it reported.
fn load(name: &str) -> (Manifest, Vec<String>) {
    let source = Source::from_path(fixture(name).join(MANIFEST_FILE)).unwrap();
    let mut sources = Sources::new();
    let id = sources.insert(source).unwrap();

    let mut diagnostics = workspace::Diagnostics::default();
    let mut source_loader = FileSourceLoader::new();
    let mut manifest = Manifest::default();

    ManifestLoader::new(
        id,
        &mut sources,
        &mut diagnostics,
        &mut source_loader,
        &mut manifest,
    )
    .load_manifest()
    .unwrap();

    let messages = diagnostics
        .diagnostics()
        .iter()
        .map(|d| {
            let workspace::Diagnostic::Fatal(f) = d;
            f.error().to_string()
        })
        .collect();

    (manifest, messages)
}

fn canonical(path: impl AsRef<Path>) -> PathBuf {
    path.as_ref().canonicalize().unwrap()
}

/// What a dependency brings is its own declaration sources and those of what
/// it depends on, each once however many ways it is reached. Without the
/// transitive ones a declaration naming a type of its own dependency would not
/// resolve, and without the deduplication every item reached twice would be
/// declared twice.
#[test]
fn a_dependency_brings_its_declarations_and_those_it_depends_on() {
    let (manifest, messages) = load("app");
    assert_eq!(messages, Vec::<String>::new());

    let [package] = &manifest.packages[..] else {
        panic!("Expected one package: {manifest:?}");
    };

    let mut names = package
        .dependencies
        .iter()
        .map(|d| d.name.as_str())
        .collect::<Vec<_>>();

    names.sort();
    assert_eq!(names, ["base", "host"]);

    let mut declarations = package.declarations.iter().cloned().collect::<Vec<_>>();
    declarations.sort();

    let mut expected = [
        canonical(fixture("host/host.d.rn")),
        canonical(fixture("base/rune/base.d.rn")),
    ];
    expected.sort();

    assert_eq!(declarations, expected);
}

/// Compile the entry point of the `app` fixture, with or without the
/// declarations its dependencies bring.
fn compile_app(with_declarations: bool) -> (bool, Vec<String>) {
    let (manifest, _) = load("app");
    let package = &manifest.packages[0];

    let mut sources = Sources::new();
    sources
        .insert(Source::from_path(fixture("app/src/main.rn")).unwrap())
        .unwrap();

    if with_declarations {
        for path in package.declarations.iter() {
            sources.insert(Source::from_path(path).unwrap()).unwrap();
        }
    }

    let context = Context::new();
    let mut diagnostics = Diagnostics::new();

    let result = prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build();

    let mut messages = Vec::new();

    for d in diagnostics.into_diagnostics() {
        if let Diagnostic::Fatal(e) = d {
            if let FatalDiagnosticKind::CompileError(e) = e.into_kind() {
                messages.push(e.to_string());
            }
        }
    }

    (result.is_ok(), messages)
}

/// A package calling what its dependencies declare compiles when their
/// declarations are compiled beside it, and does not without them, which is
/// what shows that the dependency is what makes the difference.
#[test]
fn what_a_dependency_declares_resolves() {
    let (ok, messages) = compile_app(true);
    assert!(ok, "{messages:?}");

    let (ok, messages) = compile_app(false);
    assert!(!ok);
    assert!(!messages.is_empty());
}

/// Two packages which depend on each other are reported rather than loaded
/// forever.
#[test]
fn a_dependency_cycle_is_a_diagnostic() {
    let (manifest, messages) = load("cycle-a");

    assert!(
        messages
            .iter()
            .any(|m| m.contains("Dependency `a` depends on a package which depends on it")),
        "{messages:?}"
    );

    assert_eq!(manifest.packages.len(), 1);
}

/// Only path dependencies exist, so anything else is reported where it is
/// written rather than silently ignored, while the dependencies which are fine
/// are still loaded.
#[test]
fn only_path_dependencies_are_supported() {
    let (manifest, messages) = load("bad");

    let expect = [
        "Dependency `versioned` is not supported",
        "Dependency `git` is not supported",
        "Dependency `missing` has no manifest",
        "Key `version` not supported",
    ];

    for expect in expect {
        assert!(
            messages.iter().any(|m| m.contains(expect)),
            "{expect}: {messages:?}"
        );
    }

    assert_eq!(messages.len(), expect.len(), "{messages:?}");

    let package = &manifest.packages[0];
    let names = package
        .dependencies
        .iter()
        .map(|d| d.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, ["extra"]);
    assert_eq!(
        &package.declarations[..],
        [canonical(fixture("base/rune/base.d.rn"))]
    );
}

/// A package's own declaration sources are compiled beside its entry points
/// ahead of those its dependencies bring, which is how a project describes the
/// host it is run in. One under `src/bin` is not mistaken for a binary.
#[test]
fn a_package_brings_its_own_declarations() {
    let (manifest, messages) = load("own");
    assert_eq!(messages, Vec::<String>::new());

    let [package] = &manifest.packages[..] else {
        panic!("Expected one package: {manifest:?}");
    };

    let declarations = package
        .declarations
        .iter()
        .map(canonical)
        .collect::<Vec<_>>();

    assert_eq!(
        declarations,
        [
            canonical(fixture("own/rune/own.d.rn")),
            canonical(fixture("own/src/bin/tool.d.rn")),
            canonical(fixture("base/rune/base.d.rn")),
        ]
    );

    let found = package
        .find_all(workspace::WorkspaceFilter::All)
        .unwrap()
        .into_iter()
        .map(|f| canonical(f.path))
        .collect::<Vec<_>>();

    assert_eq!(found, [canonical(fixture("own/src/main.rn"))]);
}
