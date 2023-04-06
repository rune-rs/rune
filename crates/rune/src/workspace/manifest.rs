use std::path::{PathBuf, Path};
use std::io;
use std::fs;
use std::ffi::OsStr;

use relative_path::{RelativePathBuf, RelativePath};
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

    /// Find every single entrypoint available.
    pub fn find_all(&self, m: WorkspaceFilter<'_>) -> io::Result<Vec<Found<'_>>> {
        let mut output = Vec::new();
        output.extend(self.find_bins(m)?);
        output.extend(self.find_tests(m)?);
        output.extend(self.find_examples(m)?);
        output.extend(self.find_benches(m)?);
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

pub(crate) struct Inner<'a> {
    sources: &'a mut Sources,
    diagnostics: &'a mut Diagnostics,
    manifest: &'a mut Manifest,
}

pub(crate) struct Loader<'a> {
    id: SourceId,
    inner: Inner<'a>,
}

impl<'a> Loader<'a> {
    pub(crate) fn new(id: SourceId, sources: &'a mut Sources, diagnostics: &'a mut Diagnostics, manifest: &'a mut Manifest) -> Self {
        Self {
            id,
            inner: Inner {
                sources,
                diagnostics,
                manifest,
            },
        }
    }

    /// Load a manifest.
    pub(crate) fn load_manifest(&mut self) {
        let Some(source) = self.inner.sources.get(self.id) else {
            self.fatal(WorkspaceError::new(Span::empty(), WorkspaceErrorKind::MissingSourceId { source_id: self.id }));
            return;
        };

        let value: SpannedValue = match toml::from_str(source.as_str()) {
            Ok(value) => value,
            Err(e) => {
                let span = Span::new(0, source.len());
                self.fatal(WorkspaceError::new(span, e));
                return;
            }
        };

        let root = source.path().and_then(|p| Some(p.parent()?.to_owned()));
        let root = root.as_deref();

        let Some((mut table, _)) = self.ensure_table(value) else {
            return;
        };

        // If manifest is a package, add it here.
        if let Some((package, span)) = table.remove("package").and_then(|value| self.ensure_table(value)) {
            if let Some(package) = self.load_package(package, span, root) {
                self.inner.manifest.packages.push(package);
            }
        }

        // Load the [workspace] section.
        if let Some((mut table, span)) = table.remove("workspace").and_then(|value| self.ensure_table(value)) {
            match &root {
                Some(root) => {
                    if let Some(members) = self.load_members(&mut table, root) {
                        for (span, path) in members {
                            self.load_member(span, &path);
                        }
                    }
                },
                None => {
                    self.fatal(WorkspaceError::new(span, WorkspaceErrorKind::MissingManifestPath));
                }
            }

            self.ensure_empty(table);
        }

        self.ensure_empty(table);
    }

    /// Load members from the given workspace configuration.
    fn load_members(&mut self, table: &mut Table, root: &Path) -> Option<Vec<(Span, PathBuf)>> {
        let Some(members) = table.remove("members") else {
            return None;
        };

        let (members, _) = self.ensure_array(members)?;
        let mut output = Vec::new();

        for value in members {
            let span = Spanned::span(&value);

            match deserialize::<RelativePathBuf>(value) {
                Ok(member) => {
                    self.glob_relative_path(&mut output, span, &member, root);
                }
                Err(error) => {
                    self.fatal(error);
                }
            };
        }

        Some(output)
    }

    /// Glob a relative path.
    ///
    /// Currently only supports expanding `*` and required interacting with the
    /// filesystem.
    fn glob_relative_path(&mut self, output: &mut Vec<(Span, PathBuf)>, span: Span, member: &RelativePath, root: &Path) {
        let glob = crate::workspace::glob::Glob::new(root, member);

        for m in glob.matcher() {
            let Some(mut path) = self.source_error(span, root, m) else {
                continue;
            };

            path.push(MANIFEST_FILE);

            if !path.is_file() {
                continue;
            }

            output.push((span, path));
        }
    }

    /// Helper to convert an [io::Error] into a [WorkspaceErrorKind::SourceError].
    fn source_error<T>(&mut self, span: Span, path: &Path, result: io::Result<T>) -> Option<T> {
        match result {
            Ok(result) => Some(result),
            Err(error) => {
                self.fatal(WorkspaceError::new(span, WorkspaceErrorKind::SourceError {
                    path: path.into(),
                    error,
                }));

                None
            }
        }
    }

    /// Try to load the given path as a member in the current manifest.
    fn load_member(&mut self, span: Span, path: &Path) {
        let source = match self.source_error(span, path, Source::from_path(path)) {
            Some(source) => source,
            None => return,
        };

        let id = self.inner.sources.insert(source);
        let old = std::mem::replace(&mut self.id, id);
        self.load_manifest();
        self.id = old;
    }

    /// Load a package from a value.
    fn load_package(&mut self, mut table: Table, span: Span, root: Option<&Path>) -> Option<Package> {
        let name = self.field(&mut table, span, "name");
        let version = self.field(&mut table, span, "version");
        self.ensure_empty(table);

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
    fn ensure_empty(&mut self, table: Table) {
        for (key, _) in table {
            let span = Spanned::span(&key);
            self.fatal(WorkspaceError::new(span, WorkspaceErrorKind::UnsupportedKey));
        }
    }

    /// Ensure that value is a table.
    fn ensure_table(&mut self, value: SpannedValue) -> Option<(Table, Span)> {
        let span = Spanned::span(&value);

        match value.into_inner() {
            Value::Table(table) => Some((table, span)),
            _ => {
                let error = WorkspaceError::new(span, WorkspaceErrorKind::ExpectedTable);
                self.fatal(error);
                None
            }
        }
    }

    /// Coerce into an array or error.
    fn ensure_array(&mut self, value: SpannedValue) -> Option<(Array, Span)> {
        let span = Spanned::span(&value);

        match value.into_inner() {
            Value::Array(array) => Some((array, span)),
            _ => {
                let error = WorkspaceError::expected_array(span);
                self.fatal(error);
                None
            }
        }
    }

    /// Helper to load a single field.
    fn field<T>(&mut self, table: &mut Table, span: Span, field: &'static str) -> Option<T> where T: for<'de> Deserialize<'de> {
        match table.remove(field) {
            Some(value) => {
                match deserialize(value) {
                    Ok(value) => Some(value),
                    Err(error) => {
                        self.fatal(error);
                        None
                    }
                }
            },
            None => {
                let error = WorkspaceError::missing_field(span, field);
                self.fatal(error);
                None
            }
        }
    }

    /// Report a fatal diagnostic.
    fn fatal(&mut self, error: WorkspaceError) {
        self.inner.diagnostics.fatal(self.id, error);
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
