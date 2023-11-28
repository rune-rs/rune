use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::io;
use std::iter;
use std::path::{Path, PathBuf};

use anyhow::Result;
use relative_path::{RelativePath, RelativePathBuf};
use semver::Version;
use serde::de::IntoDeserializer;
use serde::Deserialize;
use serde_hashkey as key;

use crate::alloc::prelude::*;
use crate::alloc::{self, String, Vec};
use crate::ast::{Span, Spanned};
use crate::workspace::spanned_value::{Array, SpannedValue, Table, Value};
use crate::workspace::{
    glob, Diagnostics, SourceLoader, WorkspaceError, WorkspaceErrorKind, MANIFEST_FILE,
};
use crate::{SourceId, Sources};

const BIN: &str = "bin";
const TESTS: &str = "tests";
const EXAMPLES: &str = "examples";
const BENCHES: &str = "benches";

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

/// The kind of a found entry.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum FoundKind {
    /// The found entry is a binary.
    Binary,
    /// The found entry is a test.
    Test,
    /// The found entry is an example.
    Example,
    /// The found entry is a benchmark.
    Bench,
}

impl fmt::Display for FoundKind {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FoundKind::Binary => "bin".fmt(f),
            FoundKind::Test => "test".fmt(f),
            FoundKind::Example => "example".fmt(f),
            FoundKind::Bench => "bench".fmt(f),
        }
    }
}

/// A found item in the workspace.
#[derive(Debug)]
#[non_exhaustive]
pub struct Found {
    /// The kind found.
    pub kind: FoundKind,
    /// A found path that can be built.
    pub path: PathBuf,
    /// Name of the found thing.
    pub name: String,
}

/// A found item in the workspace associated with a package.
#[derive(Debug)]
#[non_exhaustive]
pub struct FoundPackage<'a> {
    /// A found path that can be built.
    pub found: Found,
    /// Index of the package build belongs to.
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
    fn find_paths<'m>(
        &'m self,
        m: WorkspaceFilter<'_>,
        kind: FoundKind,
        auto_path: &Path,
        auto_find: fn(&Package) -> bool,
    ) -> Result<Vec<FoundPackage<'m>>> {
        let mut output = Vec::new();

        for package in self.packages.iter() {
            for found in package.find_paths(m, kind, auto_path, auto_find)? {
                output.try_push(FoundPackage { found, package })?;
            }
        }

        Ok(output)
    }

    /// Find every single entrypoint available.
    pub fn find_all(&self, m: WorkspaceFilter<'_>) -> Result<Vec<FoundPackage<'_>>> {
        let mut output = Vec::new();
        output.try_extend(self.find_bins(m)?)?;
        output.try_extend(self.find_tests(m)?)?;
        output.try_extend(self.find_examples(m)?)?;
        output.try_extend(self.find_benches(m)?)?;
        Ok(output)
    }

    /// Find all binaries matching the given name in the workspace.
    pub fn find_bins(&self, m: WorkspaceFilter<'_>) -> Result<Vec<FoundPackage<'_>>> {
        self.find_paths(m, FoundKind::Binary, Path::new(BIN), |p| p.auto_bins)
    }

    /// Find all tests associated with the given base name.
    pub fn find_tests(&self, m: WorkspaceFilter<'_>) -> Result<Vec<FoundPackage<'_>>> {
        self.find_paths(m, FoundKind::Test, Path::new(TESTS), |p| p.auto_tests)
    }

    /// Find all examples matching the given name in the workspace.
    pub fn find_examples(&self, m: WorkspaceFilter<'_>) -> Result<Vec<FoundPackage<'_>>> {
        self.find_paths(m, FoundKind::Example, Path::new(EXAMPLES), |p| {
            p.auto_examples
        })
    }

    /// Find all benches matching the given name in the workspace.
    pub fn find_benches(&self, m: WorkspaceFilter<'_>) -> Result<Vec<FoundPackage<'_>>> {
        self.find_paths(m, FoundKind::Bench, Path::new(BENCHES), |p| p.auto_benches)
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

impl Package {
    fn find_paths(
        &self,
        m: WorkspaceFilter<'_>,
        kind: FoundKind,
        auto_path: &Path,
        auto_find: fn(&Package) -> bool,
    ) -> Result<Vec<Found>> {
        let mut output = Vec::new();

        if let (Some(path), true) = (&self.root, auto_find(self)) {
            let path = path.join(auto_path);
            let results = find_rune_files(&path)?;

            for result in results {
                let (path, name) = result?;

                if m.matches(&name) {
                    output.try_push(Found { kind, path, name })?;
                }
            }
        }

        Ok(output)
    }

    /// Find every single entrypoint available.
    pub fn find_all(&self, m: WorkspaceFilter<'_>) -> Result<Vec<Found>> {
        let mut output = Vec::new();
        output.try_extend(self.find_bins(m)?)?;
        output.try_extend(self.find_tests(m)?)?;
        output.try_extend(self.find_examples(m)?)?;
        output.try_extend(self.find_benches(m)?)?;
        Ok(output)
    }

    /// Find all binaries matching the given name in the workspace.
    pub fn find_bins(&self, m: WorkspaceFilter<'_>) -> Result<Vec<Found>> {
        self.find_paths(m, FoundKind::Binary, Path::new(BIN), |p| p.auto_bins)
    }

    /// Find all tests associated with the given base name.
    pub fn find_tests(&self, m: WorkspaceFilter<'_>) -> Result<Vec<Found>> {
        self.find_paths(m, FoundKind::Test, Path::new(TESTS), |p| p.auto_tests)
    }

    /// Find all examples matching the given name in the workspace.
    pub fn find_examples(&self, m: WorkspaceFilter<'_>) -> Result<Vec<Found>> {
        self.find_paths(m, FoundKind::Example, Path::new(EXAMPLES), |p| {
            p.auto_examples
        })
    }

    /// Find all benches matching the given name in the workspace.
    pub fn find_benches(&self, m: WorkspaceFilter<'_>) -> Result<Vec<Found>> {
        self.find_paths(m, FoundKind::Bench, Path::new(BENCHES), |p| p.auto_benches)
    }
}

pub(crate) struct Loader<'a> {
    id: SourceId,
    sources: &'a mut Sources,
    diagnostics: &'a mut Diagnostics,
    source_loader: &'a mut dyn SourceLoader,
    manifest: &'a mut Manifest,
}

impl<'a> Loader<'a> {
    pub(crate) fn new(
        id: SourceId,
        sources: &'a mut Sources,
        diagnostics: &'a mut Diagnostics,
        source_loader: &'a mut dyn SourceLoader,
        manifest: &'a mut Manifest,
    ) -> Self {
        Self {
            id,
            sources,
            diagnostics,
            source_loader,
            manifest,
        }
    }

    /// Load a manifest.
    pub(crate) fn load_manifest(&mut self) -> Result<()> {
        let Some(source) = self.sources.get(self.id) else {
            self.fatal(WorkspaceError::new(
                Span::empty(),
                WorkspaceErrorKind::MissingSourceId { source_id: self.id },
            ))?;
            return Ok(());
        };

        let value: SpannedValue = match toml::from_str(source.as_str()) {
            Ok(value) => value,
            Err(e) => {
                let span = match e.span() {
                    Some(span) => Span::new(span.start, span.end),
                    None => Span::new(0, source.len()),
                };

                self.fatal(WorkspaceError::new(span, e))?;
                return Ok(());
            }
        };

        let root = source
            .path()
            .and_then(|p| p.parent().map(TryToOwned::try_to_owned))
            .transpose()?;
        let root = root.as_deref();

        let Some((mut table, _)) = self.ensure_table(value)? else {
            return Ok(());
        };

        // If manifest is a package, add it here.
        if let Some((package, span)) = table
            .remove("package")
            .map(|value| self.ensure_table(value))
            .transpose()?
            .flatten()
        {
            if let Some(package) = self.load_package(package, span, root)? {
                self.manifest.packages.try_push(package)?;
            }
        }

        // Load the [workspace] section.
        if let Some((mut table, span)) = table
            .remove("workspace")
            .map(|value| self.ensure_table(value))
            .transpose()?
            .flatten()
        {
            match &root {
                Some(root) => {
                    if let Some(members) = self.load_members(&mut table, root)? {
                        for (span, path) in members {
                            self.load_member(span, &path)?;
                        }
                    }
                }
                None => {
                    self.fatal(WorkspaceError::new(
                        span,
                        WorkspaceErrorKind::MissingManifestPath,
                    ))?;
                }
            }

            self.ensure_empty(table)?;
        }

        self.ensure_empty(table)?;
        Ok(())
    }

    /// Load members from the given workspace configuration.
    fn load_members(
        &mut self,
        table: &mut Table,
        root: &Path,
    ) -> Result<Option<Vec<(Span, PathBuf)>>> {
        let Some(members) = table.remove("members") else {
            return Ok(None);
        };

        let Some((members, _)) = self.ensure_array(members)? else {
            return Ok(None);
        };

        let mut output = Vec::new();

        for value in members {
            let span = Spanned::span(&value);

            match deserialize::<RelativePathBuf>(value) {
                Ok(member) => {
                    self.glob_relative_path(&mut output, span, &member, root)?;
                }
                Err(error) => {
                    self.fatal(error)?;
                }
            };
        }

        Ok(Some(output))
    }

    /// Glob a relative path.
    ///
    /// Currently only supports expanding `*` and required interacting with the
    /// filesystem.
    fn glob_relative_path(
        &mut self,
        output: &mut Vec<(Span, PathBuf)>,
        span: Span,
        member: &RelativePath,
        root: &Path,
    ) -> Result<()> {
        let glob = glob::Glob::new(root, member)?;

        for m in glob.matcher()? {
            let Some(mut path) = self.glob_error(span, root, m)? else {
                continue;
            };

            path.push(MANIFEST_FILE);

            if !path.is_file() {
                continue;
            }

            output.try_push((span, path))?;
        }

        Ok(())
    }

    /// Helper to convert an [io::Error] into a [WorkspaceErrorKind::SourceError].
    fn glob_error<T>(
        &mut self,
        span: Span,
        path: &Path,
        result: Result<T, glob::GlobError>,
    ) -> alloc::Result<Option<T>> {
        Ok(match result {
            Ok(result) => Some(result),
            Err(error) => {
                self.fatal(WorkspaceError::new(
                    span,
                    WorkspaceErrorKind::GlobError {
                        path: path.try_into()?,
                        error,
                    },
                ))?;

                None
            }
        })
    }

    /// Try to load the given path as a member in the current manifest.
    fn load_member(&mut self, span: Span, path: &Path) -> Result<()> {
        let source = match self.source_loader.load(span, path) {
            Ok(source) => source,
            Err(error) => {
                self.fatal(error)?;
                return Ok(());
            }
        };

        let id = self.sources.insert(source)?;
        let old = std::mem::replace(&mut self.id, id);
        self.load_manifest()?;
        self.id = old;
        Ok(())
    }

    /// Load a package from a value.
    fn load_package(
        &mut self,
        mut table: Table,
        span: Span,
        root: Option<&Path>,
    ) -> alloc::Result<Option<Package>> {
        let name = self.field(&mut table, span, "name")?;
        let version = self.field(&mut table, span, "version")?;
        self.ensure_empty(table)?;

        let (Some(name), Some(version)) = (name, version) else {
            return Ok(None);
        };

        Ok(Some(Package {
            name,
            version,
            root: root.map(|p| p.into()),
            auto_bins: true,
            auto_tests: true,
            auto_examples: true,
            auto_benches: true,
        }))
    }

    /// Ensure that a table is empty and mark any additional elements as erroneous.
    fn ensure_empty(&mut self, table: Table) -> alloc::Result<()> {
        for (key, _) in table {
            let span = Spanned::span(&key);
            self.fatal(WorkspaceError::new(
                span,
                WorkspaceErrorKind::UnsupportedKey {
                    key: key.get_ref().as_str().try_into()?,
                },
            ))?;
        }

        Ok(())
    }

    /// Ensure that value is a table.
    fn ensure_table(&mut self, value: SpannedValue) -> alloc::Result<Option<(Table, Span)>> {
        let span = Spanned::span(&value);

        Ok(match value.into_inner() {
            Value::Table(table) => Some((table, span)),
            _ => {
                let error = WorkspaceError::new(span, WorkspaceErrorKind::ExpectedTable);
                self.fatal(error)?;
                None
            }
        })
    }

    /// Coerce into an array or error.
    fn ensure_array(&mut self, value: SpannedValue) -> alloc::Result<Option<(Array, Span)>> {
        let span = Spanned::span(&value);

        Ok(match value.into_inner() {
            Value::Array(array) => Some((array, span)),
            _ => {
                let error = WorkspaceError::expected_array(span);
                self.fatal(error)?;
                None
            }
        })
    }

    /// Helper to load a single field.
    fn field<T>(
        &mut self,
        table: &mut Table,
        span: Span,
        field: &'static str,
    ) -> alloc::Result<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        Ok(match table.remove(field) {
            Some(value) => match deserialize(value) {
                Ok(value) => Some(value),
                Err(error) => {
                    self.fatal(error)?;
                    None
                }
            },
            None => {
                let error = WorkspaceError::missing_field(span, field);
                self.fatal(error)?;
                None
            }
        })
    }

    /// Report a fatal diagnostic.
    fn fatal(&mut self, error: WorkspaceError) -> alloc::Result<()> {
        self.diagnostics.fatal(self.id, error)
    }
}

/// Helper to load a single field.
fn deserialize<T>(value: SpannedValue) -> Result<T, WorkspaceError>
where
    T: for<'de> Deserialize<'de>,
{
    let span = Spanned::span(&value);
    let f = key::to_key(value.get_ref()).map_err(|e| WorkspaceError::new(span, e))?;
    let deserializer = f.into_deserializer();
    let value = T::deserialize(deserializer).map_err(|e| WorkspaceError::new(span, e))?;
    Ok(value)
}

/// Find all rune files in the given path.
fn find_rune_files(path: &Path) -> Result<impl Iterator<Item = Result<(PathBuf, String)>>> {
    let mut dir = match fs::read_dir(path) {
        Ok(dir) => Some(dir),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };

    Ok(iter::from_fn(move || loop {
        let e = dir.as_mut()?.next()?;

        let e = match e {
            Ok(e) => e,
            Err(err) => return Some(Err(err.into())),
        };

        let m = match e.metadata() {
            Ok(m) => m,
            Err(err) => return Some(Err(err.into())),
        };

        if !m.is_file() {
            continue;
        }

        let path = e.path();

        let (Some(name), Some(ext)) = (path.file_stem().and_then(OsStr::to_str), path.extension())
        else {
            continue;
        };

        if ext != OsStr::new("rn") {
            continue;
        }

        let name = match String::try_from(name) {
            Ok(name) => name,
            Err(error) => return Some(Err(error.into())),
        };

        return Some(Ok((path, name)));
    }))
}
