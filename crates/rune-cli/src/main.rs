//! <img alt="rune logo" src="https://raw.githubusercontent.com/rune-rs/rune/main/assets/icon.png" />
//! <br>
//! <a href="https://github.com/rune-rs/rune"><img alt="github" src="https://img.shields.io/badge/github-rune--rs/rune-8da0cb?style=for-the-badge&logo=github" height="20"></a>
//! <a href="https://crates.io/crates/rune-cli"><img alt="crates.io" src="https://img.shields.io/crates/v/rune-cli.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20"></a>
//! <a href="https://docs.rs/rune-cli"><img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-rune--cli-66c2a5?style=for-the-badge&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20"></a>
//! <a href="https://discord.gg/v5AeNkT"><img alt="chat on discord" src="https://img.shields.io/discord/558644981137670144.svg?logo=discord&style=flat-square" height="20"></a>
//! <br>
//! Minimum support: Rust <b>1.63+</b>.
//! <br>
//! <br>
//! <a href="https://rune-rs.github.io"><b>Visit the site 🌐</b></a>
//! &mdash;
//! <a href="https://rune-rs.github.io/book/"><b>Read the book 📖</b></a>
//! <br>
//! <br>
//!
//! An interpreter for the Rune Language, an embeddable dynamic programming language for Rust.
//!
//! <br>
//!
//! ## Usage
//!
//! If you're in the repo, you can take it for a spin with:
//!
//! ```text
//! cargo run --bin rune -- scripts/hello_world.rn
//! ```
//!
//! [Rune Language]: https://rune-rs.github.io
//! [rune]: https://github.com/rune-rs/rune

use anyhow::{anyhow, Result};
use rune::compile::ParseOptionError;
use rune::termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use rune::workspace::WorkspaceFilter;
use rune::{Context, ContextError, Options};
use rune_modules::capture_io::CaptureIo;
use std::error::Error;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use tracing_subscriber::filter::EnvFilter;

mod benches;
mod check;
mod doc;
mod loader;
mod run;
mod tests;
mod visitor;

pub const VERSION: &str = include_str!(concat!(env!("OUT_DIR"), "/version.txt"));

struct Io<'a> {
    stdout: &'a mut StandardStream,
    stderr: &'a mut StandardStream,
}

#[derive(StructOpt, Debug, Clone)]
enum Command {
    /// Run checks but do not execute
    Check(check::Flags),
    /// Build documentation.
    Doc(doc::Flags),
    /// Run all tests but do not execute
    Test(tests::Flags),
    /// Run the given program as a benchmark
    Bench(benches::Flags),
    /// Run the designated script
    Run(run::Flags),
}

impl Command {
    fn propagate_related_flags(&mut self, c: &mut Config) {
        match self {
            Command::Check(..) => {}
            Command::Doc(..) => {}
            Command::Test(..) => {
                c.test = true;
            }
            Command::Bench(..) => {
                c.test = true;
            }
            Command::Run(args) => {
                args.propagate_related_flags();
            }
        }
    }

    fn describe(&self) -> &'static str {
        match self {
            Command::Check(..) => "Checking",
            Command::Doc(..) => "Building documentation",
            Command::Test(..) => "Testing",
            Command::Bench(..) => "Benchmarking",
            Command::Run(..) => "Running",
        }
    }

    fn shared(&self) -> &SharedFlags {
        match self {
            Command::Check(args) => &args.shared,
            Command::Doc(args) => &args.shared,
            Command::Test(args) => &args.shared,
            Command::Bench(args) => &args.shared,
            Command::Run(args) => &args.shared,
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

#[derive(StructOpt, Debug, Clone)]
struct SharedFlags {
    /// Enable experimental features.
    ///
    /// This makes the `std::experimental` module available to scripts.
    #[structopt(long)]
    experimental: bool,

    /// Recursively load all files in the given directory.
    #[structopt(long)]
    recursive: bool,

    /// Display warnings.
    #[structopt(long)]
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
    #[structopt(name = "option", short = "O", number_of_values = 1)]
    compiler_options: Vec<String>,

    /// Run with the following binary from a loaded manifest. This requires a
    /// `Rune.toml` manifest.
    #[structopt(long = "bin")]
    bin: Option<String>,

    /// Run with the following test from a loaded manifest. This requires a
    /// `Rune.toml` manifest.
    #[structopt(long = "test")]
    test: Option<String>,

    /// Run with the following example from a loaded manifest. This requires a
    /// `Rune.toml` manifest.
    #[structopt(long = "example")]
    example: Option<String>,

    /// Run with the following benchmark by name from a loaded manifest. This
    /// requires a `Rune.toml` manifest.
    #[structopt(long = "bench")]
    bench: Option<String>,

    /// All paths to include in the command. By default, the tool searches the
    /// current directory and some known files for candidates.
    #[structopt(parse(from_os_str))]
    paths: Vec<PathBuf>,
}

struct Package {
    /// The name of the package the path belongs to.
    name: Box<str>,
}

enum Entry {
    /// A plain path entry.
    Path(Box<Path>),
    /// A path from a specific package.
    PackagePath(Package, Box<Path>),
}

#[derive(Default)]
struct Config {
    /// Whether or not the test module should be included.
    test: bool,
    /// Whether or not to use verbose output.
    verbose: bool,
    /// The explicit paths to load.
    entries: Vec<Entry>,
}

impl SharedFlags {
    /// Construct a rune context according to the specified argument.
    fn context(&self, c: &Config) -> Result<Context, ContextError> {
        let mut context = rune_modules::default_context()?;

        if self.experimental {
            context.install(&rune_modules::experiments::module(true)?)?;
        }

        if c.test {
            context.install(&benches::test_module()?)?;
        }

        Ok(context)
    }

    /// Setup a context that captures output.
    fn context_with_capture(&self, c: &Config, io: &CaptureIo) -> Result<Context, ContextError> {
        let mut context = rune_modules::with_config(false)?;

        context.install(&rune_modules::capture_io::module(io)?)?;

        if self.experimental {
            context.install(&rune_modules::experiments::module(true)?)?;
        }

        if c.test {
            context.install(&benches::test_module()?)?;
        }

        Ok(context)
    }
}

#[derive(Debug, Clone, StructOpt)]
#[structopt(name = "rune", about = "The Rune Language Interpreter", version = VERSION)]
struct Args {
    /// Control if output is colored or not.
    ///
    /// Valid options are:
    /// * `auto` - try to detect automatically.
    /// * `ansi` - unconditionally emit ansi control codes.
    /// * `always` - always enabled.
    ///
    /// Anything else will disable coloring.
    #[structopt(short = "C", long, default_value = "auto")]
    color: String,

    /// The command to execute
    #[structopt(subcommand)]
    cmd: Command,
}

impl Args {
    /// Construct compiler options from cli arguments.
    fn options(&self) -> Result<Options, ParseOptionError> {
        let mut options = Options::default();

        // Command-specific override defaults.
        match &self.cmd {
            Command::Test(_) | Command::Check(_) => {
                options.debug_info(true);
                options.test(true);
                options.bytecode(false);
            }
            Command::Bench(_) | Command::Doc(..) | Command::Run(_) => (),
        }

        for option in &self.cmd.shared().compiler_options {
            options.parse_option(option)?;
        }

        Ok(options)
    }
}

const SPECIAL_FILES: &[&str] = &[
    "main.rn",
    "lib.rn",
    "src/main.rn",
    "src/lib.rn",
    "script/main.rn",
    "script/lib.rn",
];

// Our own private ExitCode since std::process::ExitCode is nightly only.
// Note that these numbers are actually meaningful on Windows, but we don't
// care.
#[repr(i32)]
enum ExitCode {
    Success = 0,
    Failure = 1,
    VmError = 2,
}

#[tokio::main]
async fn main() {
    match try_main().await {
        Ok(exit_code) => {
            std::process::exit(exit_code as i32);
        }
        Err(error) => {
            let o = std::io::stderr();
            // ignore error because stdout / stderr might've been closed.
            let _ = format_errors(o.lock(), &error);
            std::process::exit(-1);
        }
    }
}

/// Format the given error.
fn format_errors<O>(mut o: O, error: &dyn Error) -> io::Result<()>
where
    O: io::Write,
{
    writeln!(o, "Error: {}", error)?;
    let mut source = error.source();

    while let Some(error) = source.take() {
        writeln!(o, "Caused by: {}", error)?;
        source = error.source();
    }

    Ok(())
}

async fn try_main() -> Result<ExitCode, io::Error> {
    let args = match Args::from_args_safe() {
        Ok(args) => args,
        Err(e) => {
            let code = if e.use_stderr() {
                let mut o = std::io::stderr();
                writeln!(o, "{}", e)?;
                ExitCode::Failure
            } else {
                let mut o = std::io::stdout();
                writeln!(o, "{}", e)?;
                ExitCode::Success
            };

            return Ok(code);
        }
    };

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

    match main_with_out(&mut io, args).await {
        Ok(code) => Ok(code),
        Err(error) => {
            let mut o = io.stdout.lock();
            o.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
            let result = format_errors(&mut o, error.as_ref());
            o.set_color(&ColorSpec::new())?;
            result?;
            Ok(ExitCode::Failure)
        }
    }
}

fn populate_config(io: &mut Io<'_>, c: &mut Config, args: &Args) -> Result<()> {
    c.entries.extend(
        args.cmd
            .shared()
            .paths
            .iter()
            .map(|p| Entry::Path(p.as_path().into())),
    );

    if !c.entries.is_empty() {
        return Ok(());
    }

    for file in SPECIAL_FILES {
        let path = Path::new(file);

        if path.is_file() {
            c.entries.push(Entry::Path(path.into()));
            return Ok(());
        }
    }

    let path = Path::new(rune::workspace::MANIFEST_FILE);

    if !path.is_file() {
        return Err(anyhow!(
            "Invalid usage: No input file nor project (`Rune.toml`) found"
        ));
    }

    // When building or running a workspace we need to be more verbose so that
    // users understand what exactly happens.
    c.verbose = true;

    let mut sources = rune::Sources::new();
    sources.insert(rune::Source::from_path(path)?);

    let mut diagnostics = rune::workspace::Diagnostics::new();

    let result = rune::workspace::prepare(&mut sources)
        .with_diagnostics(&mut diagnostics)
        .build();

    diagnostics.emit(io.stdout, &sources)?;

    let manifest = result?;

    if let Some(bin) = args.cmd.bins_test() {
        for found in manifest.find_bins(bin)? {
            let package = Package {
                name: found.package.name.clone(),
            };
            c.entries.push(Entry::PackagePath(package, found.path));
        }
    }

    if let Some(test) = args.cmd.tests_test() {
        for found in manifest.find_tests(test)? {
            let package = Package {
                name: found.package.name.clone(),
            };
            c.entries.push(Entry::PackagePath(package, found.path));
        }
    }

    if let Some(example) = args.cmd.examples_test() {
        for found in manifest.find_examples(example)? {
            let package = Package {
                name: found.package.name.clone(),
            };
            c.entries.push(Entry::PackagePath(package, found.path));
        }
    }

    if let Some(bench) = args.cmd.benches_test() {
        for found in manifest.find_benches(bench)? {
            let package = Package {
                name: found.package.name.clone(),
            };
            c.entries.push(Entry::PackagePath(package, found.path));
        }
    }

    Ok(())
}

async fn main_with_out(io: &mut Io<'_>, mut args: Args) -> Result<ExitCode> {
    let mut c = Config::default();
    args.cmd.propagate_related_flags(&mut c);
    populate_config(io, &mut c, &args)?;

    let entries = std::mem::take(&mut c.entries);
    let options = args.options()?;

    let what = args.cmd.describe();
    let verbose = c.verbose;
    let recursive = args.cmd.shared().recursive;

    for entry in entries {
        let path = match entry {
            Entry::Path(path) => path,
            Entry::PackagePath(p, path) => {
                if verbose {
                    let mut o = io.stderr.lock();
                    o.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
                    let result = write!(o, "{:>12}", what);
                    o.set_color(&ColorSpec::new())?;
                    result?;
                    writeln!(o, " `{}` (from {})", path.display(), p.name)?;
                }

                path
            }
        };

        for path in loader::recurse_paths(recursive, path) {
            let path = path?;

            match run_path(io, &c, &args, &options, &path).await? {
                ExitCode::Success => (),
                other => {
                    return Ok(other);
                }
            }
        }
    }

    Ok(ExitCode::Success)
}

/// Run a single path.
async fn run_path(
    io: &mut Io<'_>,
    c: &Config,
    args: &Args,
    options: &Options,
    path: &Path,
) -> Result<ExitCode> {
    match &args.cmd {
        Command::Check(flags) => check::run(io, c, flags, options, path),
        Command::Doc(flags) => doc::run(io, c, flags, options, path),
        Command::Test(flags) => {
            let capture_io = rune_modules::capture_io::CaptureIo::new();
            let context = flags.shared.context_with_capture(c, &capture_io)?;

            let load = loader::load(io, &context, args, options, path, visitor::Attribute::Test)?;

            tests::run(
                io,
                flags,
                &context,
                Some(&capture_io),
                load.unit,
                &load.sources,
                &load.functions,
            )
            .await
        }
        Command::Bench(flags) => {
            let capture_io = rune_modules::capture_io::CaptureIo::new();
            let context = flags.shared.context_with_capture(c, &capture_io)?;

            let load = loader::load(io, &context, args, options, path, visitor::Attribute::Bench)?;

            benches::run(
                io,
                flags,
                &context,
                Some(&capture_io),
                load.unit,
                &load.sources,
                &load.functions,
            )
            .await
        }
        Command::Run(flags) => {
            let context = flags.shared.context(c)?;
            let load = loader::load(io, &context, args, options, path, visitor::Attribute::None)?;
            run::run(io, c, flags, &context, load.unit, &load.sources).await
        }
    }
}
