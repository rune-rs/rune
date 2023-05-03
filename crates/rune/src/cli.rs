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
mod run;
mod tests;
mod visitor;

use std::fmt;

use anyhow::{bail, Context as _, Error, Result};
use clap::{Parser, Subcommand};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tracing_subscriber::filter::EnvFilter;

use crate::compile::{ItemBuf, ParseOptionError};
use crate::modules::capture_io::CaptureIo;
use crate::termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use crate::workspace::WorkspaceFilter;
use crate::{Context, ContextError, Options};

/// Default about splash.
const DEFAULT_ABOUT: &str = "The Rune Language Interpreter";

/// Options for building context.
#[non_exhaustive]
pub struct ContextOptions<'a> {
    /// The relevant I/O capture.
    pub capture: Option<&'a CaptureIo>,
    /// If experiments should be enabled or not.
    pub experimental: bool,
}

/// Type used to build a context.
pub type ContextBuilder = dyn FnMut(ContextOptions<'_>) -> Result<Context, ContextError>;

/// A rune-based entrypoint used for custom applications.
///
/// This can be used to construct your own rune-based environment, with a custom
/// configuration such as your own modules.
#[derive(Default)]
pub struct Entry<'a> {
    about: Option<String>,
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
    pub fn about(mut self, about: impl fmt::Display) -> Self {
        self.about = Some(about.to_string());
        self
    }

    /// Configure context to use.
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
            .expect("failed to build runtime");

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

        let choice = match args.color.as_str() {
            "always" => ColorChoice::Always,
            "ansi" => ColorChoice::AlwaysAnsi,
            "auto" => {
                if atty::is(atty::Stream::Stdout) {
                    ColorChoice::Auto
                } else {
                    ColorChoice::Never
                }
            }
            "never" => ColorChoice::Never,
            _ => ColorChoice::Auto,
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

struct EntryPoint {
    item: ItemBuf,
    paths: Vec<PathBuf>,
}

struct Io<'a> {
    stdout: &'a mut StandardStream,
    stderr: &'a mut StandardStream,
}

#[derive(Parser, Debug, Clone)]
struct CommandShared<T>
where
    T: clap::Args,
{
    #[command(flatten)]
    command: T,
    #[command(flatten)]
    shared: SharedFlags,
}

#[derive(Subcommand, Debug, Clone)]
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
}

impl Command {
    const ALL: [&str; 7] = [
        "check",
        "doc",
        "test",
        "bench",
        "run",
        "fmt",
        "languageserver",
    ];

    fn propagate_related_flags(&mut self, c: &mut Config) {
        match self {
            Command::Test(..) => {
                c.test = true;
            }
            Command::Bench(..) => {
                c.test = true;
            }
            Command::Run(args) => {
                args.command.propagate_related_flags();
            }
            _ => {}
        }
    }

    fn describe(&self) -> &'static str {
        match self {
            Command::Check(..) => "Checking",
            Command::Doc(..) => "Building documentation",
            Command::Fmt(..) => "Formatting files",
            Command::Test(..) => "Testing",
            Command::Bench(..) => "Benchmarking",
            Command::Run(..) => "Running",
            Command::LanguageServer(..) => "Running",
        }
    }

    fn shared(&self) -> &SharedFlags {
        match self {
            Command::Check(args) => &args.shared,
            Command::Doc(args) => &args.shared,
            Command::Fmt(args) => &args.shared,
            Command::Test(args) => &args.shared,
            Command::Bench(args) => &args.shared,
            Command::Run(args) => &args.shared,
            Command::LanguageServer(shared) => shared,
        }
    }

    fn bins_test(&self) -> Option<WorkspaceFilter<'_>> {
        if !matches!(
            self,
            Command::Run(..) | Command::Check(..) | Command::Doc(..)
        ) {
            return None;
        }

        let shared = self.shared();

        Some(if let Some(name) = &shared.bin {
            WorkspaceFilter::Name(name)
        } else {
            WorkspaceFilter::All
        })
    }

    fn tests_test(&self) -> Option<WorkspaceFilter<'_>> {
        if !matches!(
            self,
            Command::Test(..) | Command::Check(..) | Command::Doc(..)
        ) {
            return None;
        }

        let shared = self.shared();

        Some(if let Some(name) = &shared.test {
            WorkspaceFilter::Name(name)
        } else {
            WorkspaceFilter::All
        })
    }

    fn examples_test(&self) -> Option<WorkspaceFilter<'_>> {
        if !matches!(
            self,
            Command::Run(..) | Command::Check(..) | Command::Doc(..)
        ) {
            return None;
        }

        let shared = self.shared();

        Some(if let Some(name) = &shared.example {
            WorkspaceFilter::Name(name)
        } else {
            WorkspaceFilter::All
        })
    }

    fn benches_test(&self) -> Option<WorkspaceFilter<'_>> {
        if !matches!(
            self,
            Command::Bench(..) | Command::Check(..) | Command::Doc(..)
        ) {
            return None;
        }

        let shared = self.shared();

        Some(if let Some(name) = &shared.bench {
            WorkspaceFilter::Name(name)
        } else {
            WorkspaceFilter::All
        })
    }
}

struct Package {
    /// The name of the package the path belongs to.
    name: String,
}

enum BuildPath {
    /// A plain path entry.
    Path(PathBuf),
    /// A path from a specific package.
    Package(Package, PathBuf),
}

#[derive(Default)]
struct Config {
    /// Whether or not the test module should be included.
    test: bool,
    /// Whether or not to use verbose output.
    verbose: bool,
    /// Manifest root directory.
    manifest_root: Option<PathBuf>,
    /// The explicit paths to load.
    build_paths: Vec<BuildPath>,
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
            experimental: self.experimental,
        };

        let mut context = entry.context.as_mut().context("missing context")?(opts)?;

        if let Some(capture) = capture {
            context.install(crate::modules::capture_io::module(capture)?)?;
        }

        if c.test {
            context.install(benches::test_module()?)?;
        }

        Ok(context)
    }
}

#[derive(Parser, Debug, Clone)]
#[command(name = "rune", about = None)]
struct Args {
    /// Print the version of the command.
    #[arg(long)]
    version: bool,

    /// Control if output is colored or not.
    ///
    /// Valid options are:
    /// * `auto` - try to detect automatically.
    /// * `ansi` - unconditionally emit ansi control codes.
    /// * `always` - always enabled.
    ///
    /// Anything else will disable coloring.
    #[arg(short = 'C', long, default_value = "auto")]
    color: String,

    /// The command to execute
    #[command(subcommand)]
    cmd: Option<Command>,
}

impl Args {
    /// Construct compiler options from cli arguments.
    fn options(&self) -> Result<Options, ParseOptionError> {
        let mut options = Options::default();

        // Command-specific override defaults.
        if let Some(Command::Test(..) | Command::Check(..)) = &self.cmd {
            options.debug_info(true);
            options.test(true);
            options.bytecode(false);
        }

        if let Some(cmd) = &self.cmd {
            for option in &cmd.shared().compiler_options {
                options.parse_option(option)?;
            }
        }

        Ok(options)
    }
}

#[derive(Parser, Debug, Clone)]
struct SharedFlags {
    /// Enable experimental features.
    ///
    /// This makes the `std::experimental` module available to scripts.
    #[arg(long)]
    experimental: bool,

    /// Recursively load all files in the given directory.
    #[arg(long)]
    recursive: bool,

    /// Display warnings.
    #[arg(long)]
    warnings: bool,

    /// Set the given compiler option (see `--help` for available options).
    ///
    /// memoize-instance-fn[=<true/false>] - Inline the lookup of an instance function where appropriate.
    ///
    /// link-checks[=<true/false>] - Perform linker checks which makes sure that called functions exist.
    ///
    /// debug-info[=<true/false>] - Enable or disable debug info.
    ///
    /// macros[=<true/false>] - Enable or disable macros (experimental).
    ///
    /// bytecode[=<true/false>] - Enable or disable bytecode caching (experimental).
    #[arg(name = "option", short = 'O', number_of_values = 1)]
    compiler_options: Vec<String>,

    /// Run with the following binary from a loaded manifest. This requires a
    /// `Rune.toml` manifest.
    #[arg(long = "bin")]
    bin: Option<String>,

    /// Run with the following test from a loaded manifest. This requires a
    /// `Rune.toml` manifest.
    #[arg(long = "test")]
    test: Option<String>,

    /// Run with the following example from a loaded manifest. This requires a
    /// `Rune.toml` manifest.
    #[arg(long = "example")]
    example: Option<String>,

    /// Run with the following benchmark by name from a loaded manifest. This
    /// requires a `Rune.toml` manifest.
    #[arg(long = "bench")]
    bench: Option<String>,

    /// All paths to include in the command. By default, the tool searches the
    /// current directory and some known files for candidates.
    paths: Vec<PathBuf>,
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

fn find_manifest() -> Result<(PathBuf, PathBuf)> {
    let mut path = PathBuf::new();

    loop {
        let manifest_path = path.join(crate::workspace::MANIFEST_FILE);

        if manifest_path.is_file() {
            return Ok((path, manifest_path));
        }

        path.push("..");

        if !path.is_dir() {
            bail!(
                "coult not find {} in this or parent directories",
                crate::workspace::MANIFEST_FILE
            )
        }
    }
}

fn populate_config(io: &mut Io<'_>, c: &mut Config, cmd: &Command) -> Result<()> {
    c.build_paths.extend(
        cmd.shared()
            .paths
            .iter()
            .map(|p| BuildPath::Path(p.as_path().into())),
    );

    if !c.build_paths.is_empty() {
        return Ok(());
    }

    for file in SPECIAL_FILES {
        let path = Path::new(file);

        if path.is_file() {
            c.build_paths.push(BuildPath::Path(path.into()));
            return Ok(());
        }
    }

    let (manifest_root, manifest_path) = find_manifest()?;

    // When building or running a workspace we need to be more verbose so that
    // users understand what exactly happens.
    c.verbose = true;
    c.manifest_root = Some(manifest_root);

    let mut sources = crate::Sources::new();
    sources.insert(crate::Source::from_path(&manifest_path)?);

    let mut diagnostics = crate::workspace::Diagnostics::new();

    let result = crate::workspace::prepare(&mut sources)
        .with_diagnostics(&mut diagnostics)
        .build();

    diagnostics.emit(io.stdout, &sources)?;

    let manifest = result?;

    if let Some(bin) = cmd.bins_test() {
        for found in manifest.find_bins(bin)? {
            let package = Package {
                name: found.package.name.clone(),
            };
            c.build_paths.push(BuildPath::Package(package, found.path));
        }
    }

    if let Some(test) = cmd.tests_test() {
        for found in manifest.find_tests(test)? {
            let package = Package {
                name: found.package.name.clone(),
            };
            c.build_paths.push(BuildPath::Package(package, found.path));
        }
    }

    if let Some(example) = cmd.examples_test() {
        for found in manifest.find_examples(example)? {
            let package = Package {
                name: found.package.name.clone(),
            };
            c.build_paths.push(BuildPath::Package(package, found.path));
        }
    }

    if let Some(bench) = cmd.benches_test() {
        for found in manifest.find_benches(bench)? {
            let package = Package {
                name: found.package.name.clone(),
            };
            c.build_paths.push(BuildPath::Package(package, found.path));
        }
    }

    Ok(())
}

async fn main_with_out(io: &mut Io<'_>, entry: &mut Entry<'_>, mut args: Args) -> Result<ExitCode> {
    let mut c = Config::default();

    if let Some(cmd) = &mut args.cmd {
        cmd.propagate_related_flags(&mut c);
    }

    let cmd = match &args.cmd {
        Some(cmd) => cmd,
        None => {
            let commands = Command::ALL.into_iter().collect::<Vec<_>>().join(", ");
            writeln!(io.stdout, "Expected a subcommand: {commands}")?;
            return Ok(ExitCode::Failure);
        }
    };

    populate_config(io, &mut c, cmd)?;

    let entries = std::mem::take(&mut c.build_paths);
    let options = args.options()?;

    let what = cmd.describe();
    let verbose = c.verbose;
    let recursive = cmd.shared().recursive;

    let mut entrys = Vec::new();

    for entry in &entries {
        let (item, path) = match entry {
            BuildPath::Path(path) => (ItemBuf::new(), path),
            BuildPath::Package(p, path) => {
                if verbose {
                    let mut o = io.stderr.lock();
                    o.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
                    let result = write!(o, "{:>12}", what);
                    o.set_color(&ColorSpec::new())?;
                    result?;
                    writeln!(o, " `{}` (from {})", path.display(), p.name)?;
                }

                (ItemBuf::with_crate(&p.name), path)
            }
        };

        let mut paths = Vec::new();

        for path in loader::recurse_paths(recursive, path.clone()) {
            paths.push(path?);
        }

        entrys.push(EntryPoint { item, paths });
    }

    match run_path(io, &c, cmd, entry, &options, entrys).await? {
        ExitCode::Success => (),
        other => {
            return Ok(other);
        }
    }

    Ok(ExitCode::Success)
}

/// Run a single path.
async fn run_path<I>(
    io: &mut Io<'_>,
    c: &Config,
    cmd: &Command,
    entry: &mut Entry<'_>,
    options: &Options,
    entrys: I,
) -> Result<ExitCode>
where
    I: IntoIterator<Item = EntryPoint>,
{
    match cmd {
        Command::Check(f) => {
            for e in entrys {
                for path in &e.paths {
                    match check::run(io, entry, c, &f.command, &f.shared, options, path)? {
                        ExitCode::Success => (),
                        other => return Ok(other),
                    }
                }
            }
        }
        Command::Doc(f) => return doc::run(io, entry, c, &f.command, &f.shared, options, entrys),
        Command::Fmt(flags) => {
            let mut paths = vec![];
            for e in entrys {
                for path in e.paths {
                    paths.push(path);
                }
            }

            return format::run(io, &paths, &flags.command);
        }
        Command::Test(f) => {
            for e in entrys {
                for path in &e.paths {
                    let capture = crate::modules::capture_io::CaptureIo::new();
                    let context = f.shared.context(entry, c, Some(&capture))?;

                    let load = loader::load(
                        io,
                        &context,
                        &f.shared,
                        options,
                        path,
                        visitor::Attribute::Test,
                    )?;

                    match tests::run(
                        io,
                        &f.command,
                        &context,
                        Some(&capture),
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
        }
        Command::Bench(f) => {
            for e in entrys {
                for path in &e.paths {
                    let capture_io = crate::modules::capture_io::CaptureIo::new();
                    let context = f.shared.context(entry, c, Some(&capture_io))?;

                    let load = loader::load(
                        io,
                        &context,
                        &f.shared,
                        options,
                        path,
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
        }
        Command::Run(f) => {
            let context = f.shared.context(entry, c, None)?;

            for e in entrys {
                for path in &e.paths {
                    let load = loader::load(
                        io,
                        &context,
                        &f.shared,
                        options,
                        path,
                        visitor::Attribute::None,
                    )?;

                    match run::run(io, c, &f.command, &context, load.unit, &load.sources).await? {
                        ExitCode::Success => (),
                        other => return Ok(other),
                    }
                }
            }
        }
        Command::LanguageServer(shared) => {
            let context = shared.context(entry, c, None)?;
            languageserver::run(context).await?;
        }
    }

    Ok(ExitCode::Success)
}
