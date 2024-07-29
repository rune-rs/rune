//! Helper to build customized commandline interfaces using custom rune
//! contexts.
//!
//! This can be used to:
//! * Generate documentation using types only available in your context.
//! * Build a language server, which is aware of things only available in your
//!   context.

mod benches;
mod check;
mod doc;
mod format;
mod languageserver;
mod loader;
mod naming;
mod run;
mod tests;
mod visitor;

use rust_alloc::string::String;
use rust_alloc::vec::Vec;
use std::fmt;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

use crate as rune;
use crate::alloc;
use crate::alloc::prelude::*;
use crate::workspace::{self, WorkspaceFilter};

use anyhow::{bail, Context as _, Error, Result};
use clap::{Parser, Subcommand, ValueEnum};
use tracing_subscriber::filter::EnvFilter;

use crate::compile::ParseOptionError;
use crate::modules::capture_io::CaptureIo;
use crate::termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use crate::{Context, ContextError, Hash, ItemBuf, Options};

/// Default about splash.
const DEFAULT_ABOUT: &str = "The Rune Language Interpreter";

/// Options for building context.
#[non_exhaustive]
pub struct ContextOptions<'a> {
    /// If we need to capture I/O this is set to the capture instance you should
    /// be using to do so.
    pub capture: Option<&'a CaptureIo>,
    /// If we're running in a test context.
    pub test: bool,
}

/// Type used to build a context.
pub type ContextBuilder = dyn FnMut(ContextOptions<'_>) -> Result<Context, ContextError>;

/// A rune-based entrypoint used for custom applications.
///
/// This can be used to construct your own rune-based environment, with a custom
/// configuration such as your own modules.
#[derive(Default)]
pub struct Entry<'a> {
    about: Option<alloc::String>,
    context: Option<&'a mut ContextBuilder>,
}

impl<'a> Entry<'a> {
    /// Entry point.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set about string used in cli output.
    ///
    /// For example, this is the first row outputted when the command prints its
    /// help text.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// rune::cli::Entry::new()
    ///     .about("My own interpreter")
    ///     .run();
    ///```
    pub fn about(mut self, about: impl fmt::Display) -> Self {
        self.about = Some(
            about
                .try_to_string()
                .expect("Failed to format about string"),
        );
        self
    }

    /// Configure context to use using a builder.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rune::{Context, ContextError, Module};
    ///
    /// fn my_module() -> Result<Module, ContextError> {
    ///     let module = Module::default();
    ///     /* install things into module */
    ///     Ok(module)
    /// }
    ///
    /// rune::cli::Entry::new()
    ///     .about("My own interpreter")
    ///     .context(&mut |opts| {
    ///         let mut c = Context::with_config(opts.capture.is_none())?;
    ///         c.install(my_module()?);
    ///         Ok(c)
    ///     })
    ///     .run();
    ///```
    pub fn context(mut self, context: &'a mut ContextBuilder) -> Self {
        self.context = Some(context);
        self
    }

    /// Run the configured application.
    ///
    /// This will take over stdout and stdin.
    pub fn run(self) -> ! {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to build runtime");

        match runtime.block_on(self.inner()) {
            Ok(exit_code) => {
                std::process::exit(exit_code as i32);
            }
            Err(error) => {
                let o = std::io::stderr();
                // ignore error because stdout / stderr might've been closed.
                let _ = format_errors(o.lock(), &error);
                std::process::exit(ExitCode::Failure as i32);
            }
        }
    }

    /// Run the configured application without starting a new tokio runtime.
    ///
    /// This will take over stdout and stdin.
    pub async fn run_async(self) -> ! {
        match self.inner().await {
            Ok(exit_code) => {
                std::process::exit(exit_code as i32);
            }
            Err(error) => {
                let o = std::io::stderr();
                // ignore error because stdout / stderr might've been closed.
                let _ = format_errors(o.lock(), &error);
                std::process::exit(ExitCode::Failure as i32);
            }
        }
    }

    async fn inner(mut self) -> Result<ExitCode> {
        let args = match Args::try_parse() {
            Ok(args) => args,
            Err(e) => {
                let about = self.about.as_deref().unwrap_or(DEFAULT_ABOUT);

                let code = if e.use_stderr() {
                    let o = std::io::stderr();
                    let mut o = o.lock();
                    o.write_all(about.as_bytes())?;
                    writeln!(o)?;
                    writeln!(o)?;
                    writeln!(o, "{}", e)?;
                    o.flush()?;
                    ExitCode::Failure
                } else {
                    let o = std::io::stdout();
                    let mut o = o.lock();
                    o.write_all(about.as_bytes())?;
                    writeln!(o)?;
                    writeln!(o)?;
                    writeln!(o, "{}", e)?;
                    o.flush()?;
                    ExitCode::Success
                };

                return Ok(code);
            }
        };

        if args.version {
            let o = std::io::stdout();
            let mut o = o.lock();
            let about = self.about.as_deref().unwrap_or(DEFAULT_ABOUT);
            o.write_all(about.as_bytes())?;
            o.flush()?;
            return Ok(ExitCode::Success);
        }

        let choice = match args.color {
            ColorArgument::Always => ColorChoice::Always,
            ColorArgument::Ansi => ColorChoice::AlwaysAnsi,
            ColorArgument::Auto => {
                if std::io::stdin().is_terminal() {
                    ColorChoice::Auto
                } else {
                    ColorChoice::Never
                }
            }
            ColorArgument::Never => ColorChoice::Never,
        };

        let mut stdout = StandardStream::stdout(choice);
        let mut stderr = StandardStream::stderr(choice);

        let mut io = Io {
            stdout: &mut stdout,
            stderr: &mut stderr,
        };

        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .init();

        match main_with_out(&mut io, &mut self, args).await {
            Ok(code) => Ok(code),
            Err(error) => {
                let mut o = io.stdout.lock();
                o.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
                let result = format_errors(&mut o, &error);
                o.set_color(&ColorSpec::new())?;
                result?;
                Ok(ExitCode::Failure)
            }
        }
    }
}

/// A single entrypoint that can be built or processed.
#[derive(TryClone)]
pub(crate) enum EntryPoint<'a> {
    /// A plain path entrypoint.
    Path(PathBuf, bool),
    /// A package entrypoint.
    Package(workspace::FoundPackage<'a>),
}

impl EntryPoint<'_> {
    /// Path to entrypoint.
    pub(crate) fn path(&self) -> &Path {
        match self {
            EntryPoint::Path(path, _) => path,
            EntryPoint::Package(p) => &p.found.path,
        }
    }

    /// If a path is an additional argument.
    pub(crate) fn is_argument(&self) -> bool {
        match self {
            EntryPoint::Path(_, explicit) => *explicit,
            EntryPoint::Package(..) => false,
        }
    }
}

impl fmt::Display for EntryPoint<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EntryPoint::Path(path, false) => {
                write!(f, "path in {}", path.display())
            }
            EntryPoint::Path(path, true) => {
                write!(f, "path in {} (argument)", path.display())
            }
            EntryPoint::Package(package) => {
                write!(
                    f,
                    "package `{}` in {} ({})",
                    package.package.name,
                    package.found.path.display(),
                    package.found.kind
                )
            }
        }
    }
}

struct Io<'a> {
    stdout: &'a mut StandardStream,
    stderr: &'a mut StandardStream,
}

#[derive(Parser, Debug, Clone)]
#[command(rename_all = "kebab-case")]
struct CommandShared<T>
where
    T: clap::Args,
{
    #[command(flatten)]
    shared: SharedFlags,
    #[command(flatten)]
    command: T,
}

impl<T> CommandShared<T>
where
    T: CommandBase + clap::Args,
{
    /// Construct compiler options from arguments.
    fn options(&self) -> Result<Options, ParseOptionError> {
        let mut options = Options::default();

        // Command-specific override defaults.
        if self.command.is_debug() {
            options.debug_info(true);
            options.test(true);
            options.bytecode(false);
        }

        for option in &self.shared.compiler_option {
            options.parse_option(option)?;
        }

        Ok(options)
    }
}

#[derive(Clone, Copy)]
struct CommandSharedRef<'a> {
    shared: &'a SharedFlags,
    command: &'a dyn CommandBase,
}

impl CommandSharedRef<'_> {
    fn find_bins(&self, all_targets: bool) -> Option<WorkspaceFilter<'_>> {
        if !all_targets && !self.command.is_workspace(AssetKind::Bin) {
            return None;
        }

        Some(if let Some(name) = &self.shared.bin {
            WorkspaceFilter::Name(name)
        } else {
            WorkspaceFilter::All
        })
    }

    fn find_tests(&self, all_targets: bool) -> Option<WorkspaceFilter<'_>> {
        if !all_targets && !self.command.is_workspace(AssetKind::Test) {
            return None;
        }

        Some(if let Some(name) = &self.shared.test {
            WorkspaceFilter::Name(name)
        } else {
            WorkspaceFilter::All
        })
    }

    fn find_examples(&self, all_targets: bool) -> Option<WorkspaceFilter<'_>> {
        if !all_targets && !self.command.is_workspace(AssetKind::Bin) {
            return None;
        }

        Some(if let Some(name) = &self.shared.example {
            WorkspaceFilter::Name(name)
        } else {
            WorkspaceFilter::All
        })
    }

    fn find_benches(&self, all_targets: bool) -> Option<WorkspaceFilter<'_>> {
        if !all_targets && !self.command.is_workspace(AssetKind::Bench) {
            return None;
        }

        Some(if let Some(name) = &self.shared.bench {
            WorkspaceFilter::Name(name)
        } else {
            WorkspaceFilter::All
        })
    }
}

#[derive(Parser, Debug)]
#[command(rename_all = "kebab-case")]
struct HashFlags {
    /// Generate a random hash.
    #[arg(long)]
    random: bool,
    /// Items to generate hashes for.
    item: Vec<String>,
}

enum AssetKind {
    Bin,
    Test,
    Bench,
}

trait CommandBase {
    /// Test if the command should perform a debug build by default.
    #[inline]
    fn is_debug(&self) -> bool {
        false
    }

    /// Test if the command should acquire workspace assets for the given asset kind.
    #[inline]
    fn is_workspace(&self, _: AssetKind) -> bool {
        false
    }

    /// Describe the current command.
    #[inline]
    fn describe(&self) -> &str {
        "Running"
    }

    /// Propagate related flags from command and config.
    #[inline]
    fn propagate(&mut self, _: &mut Config, _: &mut SharedFlags) {}

    /// Extra paths to run.
    #[inline]
    fn paths(&self) -> &[PathBuf] {
        &[]
    }
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run checks but do not execute
    Check(CommandShared<check::Flags>),
    /// Build documentation.
    Doc(CommandShared<doc::Flags>),
    /// Run all tests but do not execute
    Test(CommandShared<tests::Flags>),
    /// Run the given program as a benchmark
    Bench(CommandShared<benches::Flags>),
    /// Run the designated script
    Run(CommandShared<run::Flags>),
    /// Format the provided file
    Fmt(CommandShared<format::Flags>),
    /// Run a language server.
    LanguageServer(SharedFlags),
    /// Helper command to generate type hashes.
    Hash(HashFlags),
}

impl Command {
    const ALL: [&'static str; 8] = [
        "check",
        "doc",
        "test",
        "bench",
        "run",
        "fmt",
        "languageserver",
        "hash",
    ];

    fn as_command_base_mut(&mut self) -> Option<(&mut SharedFlags, &mut dyn CommandBase)> {
        let (shared, command): (_, &mut dyn CommandBase) = match self {
            Command::Check(shared) => (&mut shared.shared, &mut shared.command),
            Command::Doc(shared) => (&mut shared.shared, &mut shared.command),
            Command::Test(shared) => (&mut shared.shared, &mut shared.command),
            Command::Bench(shared) => (&mut shared.shared, &mut shared.command),
            Command::Run(shared) => (&mut shared.shared, &mut shared.command),
            Command::Fmt(shared) => (&mut shared.shared, &mut shared.command),
            Command::LanguageServer(..) => return None,
            Command::Hash(..) => return None,
        };

        Some((shared, command))
    }

    fn as_command_shared_ref(&self) -> Option<CommandSharedRef<'_>> {
        let (shared, command): (_, &dyn CommandBase) = match self {
            Command::Check(shared) => (&shared.shared, &shared.command),
            Command::Doc(shared) => (&shared.shared, &shared.command),
            Command::Test(shared) => (&shared.shared, &shared.command),
            Command::Bench(shared) => (&shared.shared, &shared.command),
            Command::Run(shared) => (&shared.shared, &shared.command),
            Command::Fmt(shared) => (&shared.shared, &shared.command),
            Command::LanguageServer(..) => return None,
            Command::Hash(..) => return None,
        };

        Some(CommandSharedRef { shared, command })
    }
}

enum BuildPath<'a> {
    /// A plain path entry.
    Path(&'a Path, bool),
    /// An entry from the specified package.
    Package(workspace::FoundPackage<'a>),
}

#[derive(Default)]
struct Config {
    /// Loaded build manifest.
    manifest: workspace::Manifest,
    /// Whether or not the test module should be included.
    test: bool,
    /// Whether or not to use verbose output.
    verbose: bool,
    /// Always include all targets.
    all_targets: bool,
    /// Manifest root directory.
    manifest_root: Option<PathBuf>,
    /// Immediate found paths.
    found_paths: alloc::Vec<(PathBuf, bool)>,
}

impl Config {
    /// Construct build paths from configuration.
    fn build_paths<'m>(&'m self, cmd: CommandSharedRef<'_>) -> Result<alloc::Vec<BuildPath<'m>>> {
        let mut build_paths = alloc::Vec::new();

        if !self.found_paths.is_empty() {
            build_paths.try_extend(self.found_paths.iter().map(|(p, e)| BuildPath::Path(p, *e)))?;

            if !cmd.shared.workspace {
                return Ok(build_paths);
            }
        }

        if let Some(bin) = cmd.find_bins(self.all_targets) {
            for p in self.manifest.find_bins(bin)? {
                build_paths.try_push(BuildPath::Package(p))?;
            }
        }

        if let Some(test) = cmd.find_tests(self.all_targets) {
            for p in self.manifest.find_tests(test)? {
                build_paths.try_push(BuildPath::Package(p))?;
            }
        }

        if let Some(example) = cmd.find_examples(self.all_targets) {
            for p in self.manifest.find_examples(example)? {
                build_paths.try_push(BuildPath::Package(p))?;
            }
        }

        if let Some(bench) = cmd.find_benches(self.all_targets) {
            for p in self.manifest.find_benches(bench)? {
                build_paths.try_push(BuildPath::Package(p))?;
            }
        }

        Ok(build_paths)
    }
}

impl SharedFlags {
    /// Setup build context.
    fn context(
        &self,
        entry: &mut Entry<'_>,
        c: &Config,
        capture: Option<&CaptureIo>,
    ) -> Result<Context> {
        let opts = ContextOptions {
            capture,
            test: c.test,
        };

        let mut context =
            entry
                .context
                .as_mut()
                .context("Context builder not configured with Entry::context")?(opts)?;

        if let Some(capture) = capture {
            context.install(crate::modules::capture_io::module(capture)?)?;
        }

        Ok(context)
    }
}

#[derive(Default, Debug, Clone, Copy, ValueEnum)]
enum ColorArgument {
    #[default]
    /// Automatically enable coloring if the output is a terminal.
    Auto,
    /// Force ANSI coloring.
    Ansi,
    /// Always color using the platform-specific coloring implementation.
    Always,
    /// Never color output.
    Never,
}

#[derive(Parser, Debug)]
#[command(name = "rune", about = None)]
struct Args {
    /// Print the version of the command.
    #[arg(long)]
    version: bool,

    /// Control if output is colored or not.
    #[arg(short = 'C', long, default_value = "auto")]
    color: ColorArgument,

    /// The command to execute
    #[command(subcommand)]
    cmd: Option<Command>,
}

#[derive(Parser, Debug, Clone)]
#[command(rename_all = "kebab-case")]
struct SharedFlags {
    /// Recursively load all files if a specified build `<path>` is a directory.
    #[arg(long, short = 'R')]
    recursive: bool,

    /// Display warnings.
    #[arg(long)]
    warnings: bool,

    /// Display verbose output.
    #[arg(long)]
    verbose: bool,

    /// Collect sources to operate over from the workspace.
    ///
    /// This is what happens by default, but is disabled in case any `<paths>`
    /// are specified.
    #[arg(long)]
    workspace: bool,

    /// Set the given compiler option (see `--list-options` for available options).
    #[arg(short = 'O', num_args = 1)]
    compiler_option: Vec<String>,

    /// List available compiler options.
    #[arg(long)]
    list_options: bool,

    /// Run with the following binary from a loaded manifest. This requires a
    /// `Rune.toml` manifest.
    #[arg(long)]
    bin: Option<String>,

    /// Run with the following test from a loaded manifest. This requires a
    /// `Rune.toml` manifest.
    #[arg(long)]
    test: Option<String>,

    /// Run with the following example from a loaded manifest. This requires a
    /// `Rune.toml` manifest.
    #[arg(long)]
    example: Option<String>,

    /// Run with the following benchmark by name from a loaded manifest. This
    /// requires a `Rune.toml` manifest.
    #[arg(long)]
    bench: Option<String>,

    /// Include all targets, and not just the ones which are default for the
    /// current command.
    #[arg(long)]
    all_targets: bool,

    /// Build paths to include in the command.
    ///
    /// By default, the tool searches for:
    ///
    /// * A `Rune.toml` file in a parent directory, in which case this treated
    ///   as a workspace.
    ///
    /// * In order: `main.rn`, `lib.rn`, `src/main.rn`, `src/lib.rn`,
    ///   `script/main.rn`, and `script/lib.rn`.
    #[arg(long)]
    path: Vec<PathBuf>,
}

const SPECIAL_FILES: &[&str] = &[
    "main.rn",
    "lib.rn",
    "src/main.rn",
    "src/lib.rn",
    "script/main.rn",
    "script/lib.rn",
];

// Our own private ExitCode since std::process::ExitCode is nightly only. Note
// that these numbers are actually meaningful on Windows, but we don't care.
#[repr(i32)]
enum ExitCode {
    Success = 0,
    Failure = 1,
    VmError = 2,
}

/// Format the given error.
fn format_errors<O>(mut o: O, error: &Error) -> io::Result<()>
where
    O: io::Write,
{
    writeln!(o, "Error: {}", error)?;

    for error in error.chain().skip(1) {
        writeln!(o, "Caused by: {}", error)?;
    }

    Ok(())
}

fn find_manifest() -> Option<(PathBuf, PathBuf)> {
    let mut path = PathBuf::new();

    loop {
        let manifest_path = path.join(workspace::MANIFEST_FILE);

        if manifest_path.is_file() {
            return Some((path, manifest_path));
        }

        path.push("..");

        if !path.is_dir() {
            return None;
        }
    }
}

fn populate_config(io: &mut Io<'_>, c: &mut Config, cmd: CommandSharedRef<'_>) -> Result<()> {
    c.all_targets = cmd.shared.all_targets;

    c.found_paths
        .try_extend(cmd.shared.path.iter().map(|p| (p.clone(), false)))?;

    c.found_paths
        .try_extend(cmd.command.paths().iter().map(|p| (p.clone(), true)))?;

    if !c.found_paths.is_empty() && !cmd.shared.workspace {
        return Ok(());
    }

    let Some((manifest_root, manifest_path)) = find_manifest() else {
        for file in SPECIAL_FILES {
            let path = Path::new(file);

            if path.is_file() {
                c.found_paths.try_push((path.try_to_owned()?, false))?;
                return Ok(());
            }
        }

        let special = SPECIAL_FILES.join(", ");

        bail!(
            "Could not find `{}` in this or parent directories nor any of the special files: {special}",
            workspace::MANIFEST_FILE
        )
    };

    // When building or running a workspace we need to be more verbose so that
    // users understand what exactly happens.
    c.verbose = true;
    c.manifest_root = Some(manifest_root);

    let mut sources = crate::Sources::new();
    sources.insert(crate::Source::from_path(manifest_path)?)?;

    let mut diagnostics = workspace::Diagnostics::new();

    let result = workspace::prepare(&mut sources)
        .with_diagnostics(&mut diagnostics)
        .build();

    diagnostics.emit(io.stdout, &sources)?;
    c.manifest = result?;
    Ok(())
}

async fn main_with_out(io: &mut Io<'_>, entry: &mut Entry<'_>, mut args: Args) -> Result<ExitCode> {
    let mut c = Config::default();

    if let Some((shared, base)) = args.cmd.as_mut().and_then(|c| c.as_command_base_mut()) {
        base.propagate(&mut c, shared);
    }

    let cmd = match &args.cmd {
        Some(cmd) => cmd,
        None => {
            let commands: alloc::String = Command::ALL.into_iter().try_join(", ")?;
            writeln!(io.stdout, "Expected a subcommand: {commands}")?;
            return Ok(ExitCode::Failure);
        }
    };

    let mut entries = alloc::Vec::new();

    if let Some(cmd) = cmd.as_command_shared_ref() {
        if cmd.shared.list_options {
            writeln!(
                io.stdout,
                "Available compiler options (set with -O <option>=<value>):"
            )?;
            writeln!(io.stdout)?;

            for option in Options::available() {
                io.stdout
                    .set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
                write!(io.stdout, "{}", option.key)?;
                io.stdout.reset()?;

                write!(io.stdout, "={}", option.default)?;

                if option.unstable {
                    io.stdout
                        .set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
                    write!(io.stdout, " (unstable)")?;
                    io.stdout.reset()?;
                }

                writeln!(io.stdout, ":")?;

                writeln!(io.stdout, "    Options: {}", option.options)?;
                writeln!(io.stdout)?;

                for &line in option.doc {
                    let line = line.strip_prefix(' ').unwrap_or(line);
                    writeln!(io.stdout, "    {line}")?;
                }
            }

            return Ok(ExitCode::Success);
        }

        populate_config(io, &mut c, cmd)?;

        let build_paths = c.build_paths(cmd)?;

        let what = cmd.command.describe();
        let verbose = c.verbose;
        let recursive = cmd.shared.recursive;

        for build_path in build_paths {
            match build_path {
                BuildPath::Path(path, explicit) => {
                    for path in loader::recurse_paths(recursive, path.try_to_owned()?) {
                        entries.try_push(EntryPoint::Path(path?, explicit))?;
                    }
                }
                BuildPath::Package(p) => {
                    if verbose {
                        let mut o = io.stderr.lock();
                        o.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
                        let result = write!(o, "{:>12}", what);
                        o.set_color(&ColorSpec::new())?;
                        o.flush()?;
                        result?;
                        writeln!(
                            o,
                            " {} `{}` (from {})",
                            p.found.kind,
                            p.found.path.display(),
                            p.package.name
                        )?;
                    }

                    entries.try_push(EntryPoint::Package(p))?;
                }
            }
        }
    }

    match run_path(io, &c, cmd, entry, entries).await? {
        ExitCode::Success => (),
        other => {
            return Ok(other);
        }
    }

    Ok(ExitCode::Success)
}

/// Run a single path.
async fn run_path<'p, I>(
    io: &mut Io<'_>,
    c: &Config,
    cmd: &Command,
    entry: &mut Entry<'_>,
    entries: I,
) -> Result<ExitCode>
where
    I: IntoIterator<Item = EntryPoint<'p>>,
{
    match cmd {
        Command::Check(f) => {
            let options = f.options()?;

            for e in entries {
                let mut options = options.clone();

                if e.is_argument() {
                    options.function_body = true;
                }

                match check::run(io, entry, c, &f.command, &f.shared, &options, e.path())? {
                    ExitCode::Success => (),
                    other => return Ok(other),
                }
            }
        }
        Command::Doc(f) => {
            let options = f.options()?;
            return doc::run(io, entry, c, &f.command, &f.shared, &options, entries);
        }
        Command::Fmt(f) => {
            let options = f.options()?;
            return format::run(io, entry, c, entries, &f.command, &f.shared, &options);
        }
        Command::Test(f) => {
            let options = f.options()?;

            match tests::run(io, c, &f.command, &f.shared, &options, entry, entries).await? {
                ExitCode::Success => (),
                other => return Ok(other),
            }
        }
        Command::Bench(f) => {
            let options = f.options()?;

            for e in entries {
                let mut options = options.clone();

                if e.is_argument() {
                    options.function_body = true;
                }

                let capture_io = crate::modules::capture_io::CaptureIo::new();
                let context = f.shared.context(entry, c, Some(&capture_io))?;

                let load = loader::load(
                    io,
                    &context,
                    &f.shared,
                    &options,
                    e.path(),
                    visitor::Attribute::Bench,
                )?;

                match benches::run(
                    io,
                    &f.command,
                    &context,
                    Some(&capture_io),
                    load.unit,
                    &load.sources,
                    &load.functions,
                )
                .await?
                {
                    ExitCode::Success => (),
                    other => return Ok(other),
                }
            }
        }
        Command::Run(f) => {
            let options = f.options()?;
            let context = f.shared.context(entry, c, None)?;

            for e in entries {
                let mut options = options.clone();

                if e.is_argument() {
                    options.function_body = true;
                }

                let load = loader::load(
                    io,
                    &context,
                    &f.shared,
                    &options,
                    e.path(),
                    visitor::Attribute::None,
                )?;

                let entry = if e.is_argument() {
                    Hash::EMPTY
                } else {
                    Hash::type_hash(["main"])
                };

                match run::run(io, c, &f.command, &context, load.unit, &load.sources, entry).await?
                {
                    ExitCode::Success => (),
                    other => return Ok(other),
                }
            }
        }
        Command::LanguageServer(shared) => {
            let context = shared.context(entry, c, None)?;
            languageserver::run(context).await?;
        }
        Command::Hash(args) => {
            use rand::prelude::*;

            if args.random {
                let mut rand = rand::thread_rng();
                writeln!(io.stdout, "{}", Hash::new(rand.gen::<u64>()))?;
            }

            for item in &args.item {
                let item: ItemBuf = item.parse()?;
                let hash = Hash::type_hash(&item);
                writeln!(io.stdout, "{item} => {hash}")?;
            }
        }
    }

    Ok(ExitCode::Success)
}
