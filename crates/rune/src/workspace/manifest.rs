use std::collections::VecDeque;
use std::mem;
use std::path::{PathBuf, Path};
use std::io;
use std::fs;
use std::ffi::OsStr;

use relative_path::{RelativePathBuf, RelativePath, Component};
use semver::Version;
use serde::de::{IntoDeserializer};
use serde::Deserialize;
use serde_hashkey as key;

use crate::{Sources, SourceId, Source};
use crate::ast::{Span, Spanned};
use crate::workspace::{MANIFEST_FILE, WorkspaceErrorKind, Diagnostics, WorkspaceError};
use crate::workspace::spanned_value::{Array, SpannedValue, Value, Table};

/// A workspace filter which in combination with functions such as
/// [Manifest::find_bins] can be used to selectively find things in the
/// workspace.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum WorkspaceFilter<'a> {
    /// Look for one specific named thing.
    Name(&'a str),
    /// Look for all things.
    All,
}

/// A found item in the workspace.
#[derive(Debug)]
#[non_exhaustive]
pub struct Found<'a> {
    /// A found path that can be built.
    pub path: PathBuf,
    /// The package the found path belongs to.
    pub package: &'a Package,
}

impl WorkspaceFilter<'_> {
    fn matches(self, name: &str) -> bool {
        match self {
            WorkspaceFilter::Name(expected) => name == expected,
            WorkspaceFilter::All => true,
        }
    }
}

impl<T> Spanned for toml::Spanned<T> {
    #[inline]
    fn span(&self) -> Span {
        let range = toml::Spanned::span(self);
        Span::new(range.start, range.end)
    }
}

/// The manifest of a workspace.
#[derive(Default, Debug)]
#[non_exhaustive]
pub struct Manifest {
    /// List of packages found.
    pub packages: Vec<Package>,
}

impl Manifest {
    fn find_paths(&self, m: WorkspaceFilter<'_>, auto_path: &Path, auto_find: impl Fn(&Package) -> bool) -> io::Result<Vec<Found<'_>>> {
        let mut output = Vec::new();

        for package in &self.packages {
            if let (Some(path), true) = (&package.root, auto_find(package)) {
                let path = path.join(auto_path);
                let results = find_rune_files(&path)?;

                for result in results {
                    let (base, path) = result?;

                    if m.matches(&base) {
                        output.push(Found { path, package });
                    }
                }
            }
        }

        Ok(output)
    }

    /// Find all binaries matching the given name in the workspace.
    pub fn find_bins(&self, m: WorkspaceFilter<'_>) -> io::Result<Vec<Found<'_>>> {
        self.find_paths(m, Path::new("bin"), |p| p.auto_bins)
    }

    /// Find all tests associated with the given base name.
    pub fn find_tests(&self, m: WorkspaceFilter<'_>) -> io::Result<Vec<Found<'_>>> {
        self.find_paths(m, Path::new("tests"), |p| p.auto_tests)
    }

    /// Find all examples matching the given name in the workspace.
    pub fn find_examples(&self, m: WorkspaceFilter<'_>) -> io::Result<Vec<Found<'_>>> {
        self.find_paths(m, Path::new("examples"), |p| p.auto_examples)
    }

    /// Find all benches matching the given name in the workspace.
    pub fn find_benches(&self, m: WorkspaceFilter<'_>) -> io::Result<Vec<Found<'_>>> {
        self.find_paths(m, Path::new("benches"), |p| p.auto_benches)
    }
}

/// A single package.
#[derive(Debug)]
#[non_exhaustive]
pub struct Package {
    /// The name of the package.
    pub name: String,
    /// The version of the package..
    pub version: Version,
    /// The root of the package.
    pub root: Option<PathBuf>,
    /// Automatically detect binaries.
    pub auto_bins: bool,
    /// Automatically detect tests.
    pub auto_tests: bool,
    /// Automatically detect examples.
    pub auto_examples: bool,
    /// Automatically detect benches.
    pub auto_benches: bool,
}

pub(crate) struct Loader<'a> {
    pub(crate) id: SourceId,
    pub(crate) sources: &'a mut Sources,
    pub(crate) diagnostics: &'a mut Diagnostics,
    pub(crate) manifest: &'a mut Manifest,
}

/// Load a manifest.
pub(crate) fn load_manifest(l: &mut Loader<'_>) {
    let (value, root) = match l.sources.get(l.id) {
        Some(source) => {
            let root: Option<PathBuf> = source.path().and_then(|p| p.parent()).map(|p| p.into());

            let value: SpannedValue = match toml::from_str(source.as_str()) {
                Ok(value) => value,
                Err(e) => {
                    let span = Span::new(0, source.len());
                    l.diagnostics.fatal(l.id, WorkspaceError::new(span, e));
                    return;
                }
            };

            (value, root)
        }
        None => {
            l.diagnostics.fatal(l.id, WorkspaceError::new(Span::empty(), WorkspaceErrorKind::MissingSourceId { source_id: l.id }));
            return;
        }
    };

    if let Some((mut table, _)) = into_table(l, value) {
        // If manifest is a package, add it here.
        if let Some(package) = table.remove("package") {
            if let Some((mut package, span)) = into_table(l, package) {
                if let Some(package) = load_package(l, &mut package, span, root.as_deref()) {
                    l.manifest.packages.push(package);
                }

                ensure_empty(l, package);
            }
        }

        // Load the [workspace] section.
        if let Some(workspace) = table.remove("workspace") {
            if let Some((mut table, span)) = into_table(l, workspace) {
                match &root {
                    Some(root) => {
                        if let Some(members) = load_members(l, &mut table, root) {
                            for (span, path) in members {
                                load_member(l, span, &path);
                            }
                        }
                    },
                    None => {
                        l.diagnostics.fatal(l.id, WorkspaceError::new(span, WorkspaceErrorKind::MissingManifestPath));
                    }
                }

                ensure_empty(l, table);
            }
        }

        ensure_empty(l, table);
    }
}

/// Load members from the given workspace configuration.
fn load_members(l: &mut Loader<'_>, table: &mut Table, root: &Path) -> Option<Vec<(Span, PathBuf)>> {
    let members = match table.remove("members") {
        Some(members) => members,
        None => return None,
    };

    let (members, _) = into_array(l, members)?;
    let mut output = Vec::new();

    for value in members {
        let span = Spanned::span(&value);

        match deserialize::<RelativePathBuf>(value) {
            Ok(member) => {
                glob_relative_path(l, &mut output, span, &member, root);
            }
            Err(error) => {
                l.diagnostics.fatal(l.id, error);
            }
        };
    }

    Some(output)
}

/// Glob a relative path.
///
/// Currently only supports expanding `*` and required interacting with the
/// filesystem.
fn glob_relative_path(l: &mut Loader<'_>, output: &mut Vec<(Span, PathBuf)>, span: Span, member: &RelativePath, root: &Path) {
    let mut queue = VecDeque::new();
    queue.push_back((root.to_owned(), member.components()));

    while let Some((mut path, mut it)) = queue.pop_front() {
        loop {
            let c = match it.next() {
                Some(c) => c,
                None => {
                    path.push(MANIFEST_FILE);
                    output.push((span, path));
                    break;
                }
            };

            match c {
                Component::CurDir => {},
                Component::ParentDir => {
                    path.push("..");
                },
                Component::Normal("*") => {
                    let result = match source_error(l, span, &path, fs::read_dir(&path)) {
                        Some(result) => result,
                        None => continue,
                    };

                    for e in result {
                        let e = match source_error(l, span, &path, e) {
                            Some(e) => e,
                            None => continue,
                        };

                        let path = e.path();

                        let m = match source_error(l, span, &path, e.metadata()) {
                            Some(m) => m,
                            None => continue,
                        };

                        if m.is_dir() {
                            queue.push_back((path, it.clone()));
                        }
                    }

                    break;
                },
                Component::Normal(normal) => {
                    path.push(normal);
                },
            }
        }
    }
}

/// Helper to convert an [io::Error] into a [WorkspaceErrorKind::SourceError].
fn source_error<T>(l: &mut Loader<'_>, span: Span, path: &Path, result: io::Result<T>) -> Option<T> {
    match result {
        Ok(result) => Some(result),
        Err(error) => {
            l.diagnostics.fatal(l.id, WorkspaceError::new(span, WorkspaceErrorKind::SourceError {
                path: path.into(),
                error,
            }));

            None
        }
    }
}

/// Try to load the given path as a member in the current manifest.
fn load_member(l: &mut Loader<'_>, span: Span, path: &Path) {
    let source = match source_error(l, span, path, Source::from_path(path)) {
        Some(source) => source,
        None => return,
    };

    let id = l.sources.insert(source);
    let old = mem::replace(&mut l.id, id);
    load_manifest(l);
    l.id = old;
}

/// Load a package from a value.
fn load_package(l: &mut Loader<'_>, table: &mut Table, span: Span, root: Option<&Path>) -> Option<Package> {
    let name = field(l, table, span, "name");
    let version = field(l, table, span, "version");

    Some(Package {
        name: name?,
        version: version?,
        root: root.map(|p| p.into()),
        auto_bins: true,
        auto_tests: true,
        auto_examples: true,
        auto_benches: true,
    })
}

/// Ensure that a table is empty and mark any additional elements as erroneous.
fn ensure_empty(l: &mut Loader<'_>, table: Table) {
    for (key, _) in table {
        let span = Spanned::span(&key);
        l.diagnostics.fatal(l.id, WorkspaceError::new(span, WorkspaceErrorKind::UnsupportedKey));
    }
}

/// Ensure that value is a table.
fn into_table(l: &mut Loader<'_>, value: SpannedValue) -> Option<(Table, Span)> {
    let span = Spanned::span(&value);

    match value.into_inner() {
        Value::Table(table) => Some((table, span)),
        _ => {
            let error = WorkspaceError::new(span, WorkspaceErrorKind::ExpectedTable);
            l.diagnostics.fatal(l.id, error);
            None
        }
    }
}

/// Coerce into an array or error.
fn into_array(l: &mut Loader<'_>, value: SpannedValue) -> Option<(Array, Span)> {
    let span = Spanned::span(&value);

    match value.into_inner() {
        Value::Array(array) => Some((array, span)),
        _ => {
            let error = WorkspaceError::expected_array(span);
            l.diagnostics.fatal(l.id, error);
            None
        }
    }
}

/// Helper to load a single field.
fn field<T>(l: &mut Loader<'_>, table: &mut Table, span: Span, field: &'static str) -> Option<T> where T: for<'de> Deserialize<'de> {
    match table.remove(field) {
        Some(value) => {
            match deserialize(value) {
                Ok(value) => Some(value),
                Err(error) => {
                    l.diagnostics.fatal(l.id, error);
                    None
                }
            }
        },
        None => {
            let error = WorkspaceError::missing_field(span, field);
            l.diagnostics.fatal(l.id, error);
            None
        }
    }
}

/// Helper to load a single field.
fn deserialize<T>(value: SpannedValue) -> Result<T, WorkspaceError> where T: for<'de> Deserialize<'de> {
    let span = Spanned::span(&value);
    let f = key::to_key(value.get_ref()).map_err(|e| WorkspaceError::new(span, e))?;
    let deserializer = f.into_deserializer();
    let value = T::deserialize(deserializer).map_err(|e| WorkspaceError::new(span, e))?;
    Ok(value)
}

/// Find all rune files in the given path.
fn find_rune_files(path: &Path) -> io::Result<impl Iterator<Item = io::Result<(String, PathBuf)>>> {
    let mut dir = match fs::read_dir(path) {
        Ok(dir) => Some(dir),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(e),
    };

    Ok(std::iter::from_fn(move || {
        loop {
            let e = dir.as_mut()?.next()?;

            let e = match e {
                Ok(e) => e,
                Err(err) => return Some(Err(err)),
            };

            let m = match e.metadata() {
                Ok(m) => m,
                Err(err) => return Some(Err(err)),
            };

            if !m.is_file() {
                continue;
            }

            let path = e.path();

            if let (Some(base), Some(ext)) = (path.file_stem(), path.extension()) {
                if ext == OsStr::new("rn") {
                    if let Some(base) = base.to_str() {
                        return Some(Ok((base.into(), path)));
                    }
                }
            }
        }
    }))
}
