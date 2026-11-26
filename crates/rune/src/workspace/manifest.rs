use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use relative_path::{RelativePath, RelativePathBuf};
use semver::Version;
use serde::de::IntoDeserializer;
use serde::Deserialize;
use serde_hashkey as key;

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::{self, HashMap, String, Vec};
use crate::ast::{Span, Spanned};
use crate::source::is_declaration_name;
use crate::workspace::spanned_value::{Array, SpannedValue, Table, Value};
use crate::workspace::{
    glob, Diagnostics, SourceLoader, WorkspaceError, WorkspaceErrorKind, MANIFEST_FILE,
};
use crate::{SourceId, Sources};

/// A workspace filter which in combination with [Manifest::find_by_kind]
/// can be used to selectively find things in the workspace.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum WorkspaceFilter<'a> {
    /// Look for one specific named thing.
    Name(&'a str),
    /// Look for all things.
    All,
}

/// The kind of a found entry.
#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum FoundKind {
    /// The found entry is a binary.
    Binary,
    /// The found entry is a source file.
    Library,
    /// The found entry is a test.
    Test,
    /// The found entry is an example.
    Example,
    /// The found entry is a benchmark.
    Bench,
}

impl FoundKind {
    fn all() -> [Self; 5] {
        [
            Self::Binary,
            Self::Library,
            Self::Test,
            Self::Example,
            Self::Bench,
        ]
    }
}

impl fmt::Display for FoundKind {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FoundKind::Library => "lib".fmt(f),
            FoundKind::Binary => "bin".fmt(f),
            FoundKind::Test => "test".fmt(f),
            FoundKind::Example => "example".fmt(f),
            FoundKind::Bench => "bench".fmt(f),
        }
    }
}

/// A found item in the workspace.
#[derive(Debug, TryClone)]
#[non_exhaustive]
pub struct Found {
    /// The kind found.
    #[try_clone(copy)]
    pub kind: FoundKind,
    /// A found path that can be built.
    pub path: PathBuf,
    /// Name of the found thing.
    pub name: String,
}

/// A found item in the workspace associated with a package.
#[derive(Debug, TryClone)]
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
    /// Find all entrypoints of a specific kind.
    pub fn find_by_kind(
        &self,
        filter: WorkspaceFilter<'_>,
        kind: FoundKind,
    ) -> Result<Vec<FoundPackage<'_>>> {
        let mut output = Vec::new();

        for package in self.packages.iter() {
            for found in package.find_by_kind(filter, kind)? {
                output.try_push(FoundPackage { found, package })?;
            }
        }

        Ok(output)
    }

    /// Find every single entrypoint available.
    pub fn find_all(&self, filter: WorkspaceFilter<'_>) -> Result<Vec<FoundPackage<'_>>> {
        let mut output = Vec::new();
        for kind in FoundKind::all() {
            output.try_extend(self.find_by_kind(filter, kind)?)?;
        }
        Ok(output)
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
    /// Automatically detect libraries.
    pub auto_libs: bool,
    /// Automatically detect tests.
    pub auto_tests: bool,
    /// Automatically detect examples.
    pub auto_examples: bool,
    /// Automatically detect benches.
    pub auto_benches: bool,
    /// The packages named in `[dependencies]`.
    pub dependencies: Vec<Dependency>,
    /// The declaration sources (`*.d.rn`) of the package itself and of every
    /// package it depends on, directly or not, each once. These are compiled
    /// beside each of the package's entry points.
    pub declarations: Vec<PathBuf>,
}

/// A dependency of a package, as named in its `[dependencies]`.
#[derive(Debug, TryClone)]
#[non_exhaustive]
pub struct Dependency {
    /// The name the dependency is given in the manifest.
    pub name: String,
    /// The directory of the dependency, holding its `Rune.toml`.
    pub root: PathBuf,
}

impl Package {
    fn auto_find(&self, kind: FoundKind) -> bool {
        match kind {
            FoundKind::Binary => self.auto_bins,
            FoundKind::Library => self.auto_libs,
            FoundKind::Test => self.auto_tests,
            FoundKind::Example => self.auto_examples,
            FoundKind::Bench => self.auto_benches,
        }
    }

    fn find_by_kind(&self, filter: WorkspaceFilter<'_>, kind: FoundKind) -> Result<Vec<Found>> {
        match kind {
            FoundKind::Binary => self.find_bins(filter),
            FoundKind::Library => self
                .find_lib(filter)
                .and_then(|lib| lib.into_iter().try_collect().map_err(anyhow::Error::from)),
            FoundKind::Test => self.find_tests(filter),
            FoundKind::Bench => self.find_benches(filter),
            FoundKind::Example => self.find_examples(filter),
        }
    }

    /// Find every single entrypoint available.
    pub fn find_all(&self, filter: WorkspaceFilter<'_>) -> Result<Vec<Found>> {
        let mut output = Vec::new();
        for kind in FoundKind::all() {
            output.try_extend(self.find_by_kind(filter, kind)?)?;
        }
        Ok(output)
    }

    /// Find all binaries matching the given name in the package.
    pub fn find_bins(&self, filter: WorkspaceFilter<'_>) -> Result<Vec<Found>> {
        if !self.auto_find(FoundKind::Binary) {
            return Ok(Vec::new());
        }

        let Some(root) = &self.root else {
            return Ok(Vec::new());
        };

        let src_path = root.join("src");

        let mut found = Vec::new();

        let bin_entry_point = src_path.join("main.rn");
        if bin_entry_point.exists() && bin_entry_point.is_file() && filter.matches(&self.name) {
            found.try_push(Found {
                kind: FoundKind::Binary,
                path: bin_entry_point,
                name: self.name.try_clone()?,
            })?;
        }

        let bin_directory = src_path.join("bin");
        if bin_directory.exists() && bin_directory.is_dir() {
            for (path, name) in find_binary_entry_points(&bin_directory)? {
                if filter.matches(&name) {
                    found.try_push(Found {
                        kind: FoundKind::Binary,
                        path,
                        name,
                    })?;
                }
            }
        }

        Ok(found)
    }

    /// Find a library entry point matching the given name in the package, if one exists.
    pub fn find_lib(&self, filter: WorkspaceFilter<'_>) -> Result<Option<Found>> {
        if !self.auto_find(FoundKind::Library) {
            return Ok(None);
        }

        let Some(root) = &self.root else {
            return Ok(None);
        };

        let src_path = root.join("src");

        let mut lib = None;

        let lib_entry_point = src_path.join("lib.rn");
        if lib_entry_point.exists() && lib_entry_point.is_file() {
            if !filter.matches(&self.name) {
                return Ok(None);
            }

            lib = Some(Found {
                kind: FoundKind::Library,
                path: lib_entry_point,
                name: self.name.try_clone()?,
            });
        }

        Ok(lib)
    }

    fn find_in_directory(
        &self,
        filter: WorkspaceFilter<'_>,
        kind: FoundKind,
        directory: &str,
    ) -> Result<Vec<Found>> {
        if !self.auto_find(kind) {
            return Ok(Vec::new());
        }

        let Some(root) = &self.root else {
            return Ok(Vec::new());
        };

        let directory_path = root.join(directory);
        if !directory_path.exists() || !directory_path.is_dir() {
            return Ok(Vec::new());
        }

        let mut found = Vec::new();

        for (path, name) in find_binary_entry_points(&directory_path)? {
            if filter.matches(&name) {
                found.try_push(Found { kind, path, name })?;
            }
        }

        Ok(found)
    }

    /// Find all tests associated with the given base name.
    pub fn find_tests(&self, filter: WorkspaceFilter<'_>) -> Result<Vec<Found>> {
        self.find_in_directory(filter, FoundKind::Test, "tests")
    }

    /// Find all examples matching the given name in the package.
    pub fn find_examples(&self, filter: WorkspaceFilter<'_>) -> Result<Vec<Found>> {
        self.find_in_directory(filter, FoundKind::Example, "examples")
    }

    /// Find all benches matching the given name in the workspace.
    pub fn find_benches(&self, filter: WorkspaceFilter<'_>) -> Result<Vec<Found>> {
        self.find_in_directory(filter, FoundKind::Bench, "benches")
    }
}

/// Loader for manifests
pub struct Loader<'a> {
    id: SourceId,
    sources: &'a mut Sources,
    diagnostics: &'a mut Diagnostics,
    source_loader: &'a mut dyn SourceLoader,
    manifest: &'a mut Manifest,
    /// The roots of the packages whose dependencies are being loaded, from
    /// the outermost in, which is what a cycle is noticed by.
    loading: Vec<PathBuf>,
    /// The declaration sources a dependency was found to bring, by its root,
    /// so that a package several others depend on is loaded once.
    resolved: HashMap<PathBuf, Vec<PathBuf>>,
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
            loading: Vec::new(),
            resolved: HashMap::new(),
        }
    }

    /// Parse the current source as a table, along with the directory it is
    /// in if it has a path.
    fn parse(&mut self) -> Result<Option<(Table, Option<PathBuf>)>> {
        let Some(source) = self.sources.get(self.id) else {
            self.fatal(WorkspaceError::new(
                Span::empty(),
                WorkspaceErrorKind::MissingSourceId { source_id: self.id },
            ))?;
            return Ok(None);
        };

        let value: SpannedValue = match toml::from_str(source.as_str()) {
            Ok(value) => value,
            Err(e) => {
                let span = match e.span() {
                    Some(span) => Span::new(span.start, span.end),
                    None => Span::new(0, source.len()),
                };

                self.fatal(WorkspaceError::new(span, e))?;
                return Ok(None);
            }
        };

        let root = source
            .path()
            .and_then(|p| p.parent().map(TryToOwned::try_to_owned))
            .transpose()?;

        let Some((table, _)) = self.ensure_table(value)? else {
            return Ok(None);
        };

        Ok(Some((table, root)))
    }

    /// Load a manifest.
    pub(crate) fn load_manifest(&mut self) -> Result<()> {
        let Some((mut table, root)) = self.parse()? else {
            return Ok(());
        };

        let root = root.as_deref();
        let has_package = table.contains_key("package");

        // If manifest is a package, add it here.
        let mut package = match table
            .remove("package")
            .map(|value| self.ensure_table(value))
            .transpose()?
            .flatten()
        {
            Some((package, span)) => self.load_package(package, span, root)?,
            None => None,
        };

        // A package's own declaration sources come first, the common case
        // being a project describing the host it is run in.
        if let (Some(package), Some(root)) = (&mut package, root) {
            package.declarations = self.find_declarations(root)?;
        }

        if let Some(value) = table.remove("dependencies") {
            let span = Spanned::span(&value);

            match (&mut package, root) {
                (Some(package), Some(root)) => {
                    let (dependencies, declarations) = self.load_dependencies(value, root)?;
                    package.dependencies = dependencies;

                    for path in declarations {
                        if !package.declarations.contains(&path) {
                            package.declarations.try_push(path)?;
                        }
                    }
                }
                // The package itself is broken, which has been reported.
                (None, _) if has_package => {}
                (None, _) => {
                    self.fatal(WorkspaceError::new(
                        span,
                        WorkspaceErrorKind::DependenciesWithoutPackage,
                    ))?;
                }
                (Some(..), None) => {
                    self.fatal(WorkspaceError::new(
                        span,
                        WorkspaceErrorKind::MissingManifestPath,
                    ))?;
                }
            }
        }

        if let Some(package) = package {
            self.manifest.packages.try_push(package)?;
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

    /// Load the `[dependencies]` of the package rooted at `root`, returning
    /// the dependencies and the declaration sources they bring between them.
    fn load_dependencies(
        &mut self,
        value: SpannedValue,
        root: &Path,
    ) -> Result<(Vec<Dependency>, Vec<PathBuf>)> {
        let mut dependencies = Vec::new();
        let mut declarations = Vec::new();

        let Some((table, _)) = self.ensure_table(value)? else {
            return Ok((dependencies, declarations));
        };

        self.loading.try_push(canonical(root)?)?;

        for (key, value) in table {
            let name = String::try_from(key.get_ref().as_str())?;
            let span = Spanned::span(&value);

            let Some(path) = self.dependency_path(&name, value)? else {
                continue;
            };

            let dir = path.to_path(root);
            let manifest = dir.join(MANIFEST_FILE);

            if !manifest.is_file() {
                self.fatal(WorkspaceError::new(
                    span,
                    WorkspaceErrorKind::MissingDependency {
                        name,
                        path: manifest.as_path().try_into()?,
                    },
                ))?;
                continue;
            }

            let dir = canonical(&dir)?;

            if self.loading.contains(&dir) {
                self.fatal(WorkspaceError::new(
                    span,
                    WorkspaceErrorKind::DependencyCycle { name },
                ))?;
                continue;
            }

            let found = match self.resolved.get(&dir) {
                Some(found) => found.try_clone()?,
                None => {
                    let found = self.load_dependency(span, &manifest, &dir)?;
                    self.resolved
                        .try_insert(dir.try_clone()?, found.try_clone()?)?;
                    found
                }
            };

            for path in found {
                if !declarations.contains(&path) {
                    declarations.try_push(path)?;
                }
            }

            dependencies.try_push(Dependency { name, root: dir })?;
        }

        self.loading.pop();
        Ok((dependencies, declarations))
    }

    /// The path of a dependency, which is the only kind there is.
    fn dependency_path(
        &mut self,
        name: &str,
        value: SpannedValue,
    ) -> alloc::Result<Option<RelativePathBuf>> {
        let span = Spanned::span(&value);

        let path = match value.into_inner() {
            Value::Table(mut table) => match table.remove("path") {
                Some(path) => {
                    self.ensure_empty(table)?;
                    Some(path)
                }
                None => None,
            },
            _ => None,
        };

        let Some(path) = path else {
            self.fatal(WorkspaceError::new(
                span,
                WorkspaceErrorKind::UnsupportedDependency {
                    name: name.try_to_owned()?,
                },
            ))?;
            return Ok(None);
        };

        match deserialize::<RelativePathBuf>(path) {
            Ok(path) => Ok(Some(path)),
            Err(error) => {
                self.fatal(error)?;
                Ok(None)
            }
        }
    }

    /// Load the dependency whose manifest is at `path` and whose root is
    /// `root`, returning the declaration sources it brings: its own and those
    /// of its dependencies.
    fn load_dependency(&mut self, span: Span, path: &Path, root: &Path) -> Result<Vec<PathBuf>> {
        let source = match self.source_loader.load(span, path) {
            Ok(source) => source,
            Err(error) => {
                self.fatal(error)?;
                return Ok(Vec::new());
            }
        };

        let id = self.sources.insert(source)?;
        let old = std::mem::replace(&mut self.id, id);
        let result = self.load_dependency_manifest(root);
        self.id = old;
        result
    }

    fn load_dependency_manifest(&mut self, root: &Path) -> Result<Vec<PathBuf>> {
        let Some((mut table, _)) = self.parse()? else {
            return Ok(Vec::new());
        };

        // A dependency is still checked for being a valid package, but it is
        // not a member of the workspace being loaded, so none of its entry
        // points are, and a `[workspace]` it has is its own business.
        match table
            .remove("package")
            .map(|value| self.ensure_table(value))
            .transpose()?
            .flatten()
        {
            Some((package, span)) => {
                self.load_package(package, span, Some(root))?;
            }
            None => {
                self.fatal(WorkspaceError::missing_field(Span::empty(), "package"))?;
            }
        }

        _ = table.remove("workspace");

        let transitive = match table.remove("dependencies") {
            Some(value) => self.load_dependencies(value, root)?.1,
            None => Vec::new(),
        };

        self.ensure_empty(table)?;

        let mut declarations = self.find_declarations(root)?;

        for path in transitive {
            if !declarations.contains(&path) {
                declarations.try_push(path)?;
            }
        }

        Ok(declarations)
    }

    /// The declaration sources in the package rooted at `root`, reporting a
    /// directory which cannot be read as a manifest diagnostic.
    fn find_declarations(&mut self, root: &Path) -> Result<Vec<PathBuf>> {
        // A manifest loaded as `Rune.toml` has an empty directory, which is
        // the current one.
        let dir = if root.as_os_str().is_empty() {
            Path::new(".")
        } else {
            root
        };

        match find_declarations(dir) {
            Ok(declarations) => Ok(declarations),
            Err(error) => {
                self.fatal(WorkspaceError::new(
                    Span::empty(),
                    WorkspaceErrorKind::ReadDeclarations {
                        path: root.try_into()?,
                        error,
                    },
                ))?;
                Ok(Vec::new())
            }
        }
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
            auto_libs: true,
            auto_bins: true,
            auto_tests: true,
            auto_examples: true,
            auto_benches: true,
            dependencies: Vec::new(),
            declarations: Vec::new(),
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

/// Find binary entry points in the given directory
fn find_binary_entry_points(path: &Path) -> Result<Vec<(PathBuf, String)>> {
    let mut entry_points = Vec::new();

    for entry in path.read_dir()? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        // A declaration source is compiled beside the entry points rather
        // than being one.
        if file_type.is_file()
            && entry.path().extension() == Some(OsStr::new("rn"))
            && !is_declaration_name(&entry.file_name().to_string_lossy())
        {
            entry_points.try_push((
                entry.path(),
                entry
                    .path()
                    .file_stem()
                    .ok_or_else(|| anyhow!("failed to find file stem for {:?}", entry.path()))?
                    .to_string_lossy()
                    .try_to_owned()?,
            ))?;
        } else if file_type.is_dir() {
            let main = entry.path().join("main.rn");
            if main.exists() && main.is_file() {
                entry_points.try_push((
                    main,
                    entry
                        .path()
                        .file_name()
                        .ok_or_else(|| {
                            anyhow!(
                                "failed to find trailing directory name for {:?}",
                                entry.path()
                            )
                        })?
                        .to_string_lossy()
                        .try_to_owned()?,
                ))?;
            }
        }
    }

    Ok(entry_points)
}

/// The canonical form of `path`, so that two ways of naming one package are
/// noticed as the same, or `path` itself if it cannot be resolved.
fn canonical(path: &Path) -> alloc::Result<PathBuf> {
    // A manifest loaded as `Rune.toml` has an empty directory, which is the
    // current one.
    let path = if path.as_os_str().is_empty() {
        Path::new(".")
    } else {
        path
    };

    match path.canonicalize() {
        Ok(path) => Ok(path),
        Err(..) => path.try_to_owned(),
    }
}

/// Find the declaration sources (`*.d.rn`) in the package rooted at `root`,
/// in a stable order.
///
/// Hidden directories, `target` and directories holding a package of their own
/// are not looked in, and symbolic links are not followed, so that a link back
/// up the tree does not make the walk endless.
fn find_declarations(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut output = Vec::new();
    let mut queue = Vec::new();
    queue
        .try_push(root.to_path_buf())
        .map_err(io::Error::other)?;

    while let Some(dir) = queue.pop() {
        for entry in dir.read_dir()? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let path = entry.path();

            if file_type.is_dir() {
                if name.starts_with('.') || name == "target" || path.join(MANIFEST_FILE).is_file() {
                    continue;
                }

                queue.try_push(path).map_err(io::Error::other)?;
            } else if file_type.is_file() && is_declaration_name(&name) {
                output.try_push(path).map_err(io::Error::other)?;
            }
        }
    }

    output.sort();
    Ok(output)
}
