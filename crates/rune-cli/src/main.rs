//! <div align="center">
//!     <img alt="Rune Logo" src="https://raw.githubusercontent.com/rune-rs/rune/main/assets/icon.png" />
//! </div>
//!
//! <br>
//!
//! <div align="center">
//! <a href="https://rune-rs.github.io">
//!     <b>Visit the site 🌐</b>
//! </a>
//! -
//! <a href="https://rune-rs.github.io/book/">
//!     <b>Read the book 📖</b>
//! </a>
//! </div>
//!
//! <br>
//!
//! <div align="center">
//! <a href="https://github.com/rune-rs/rune/actions">
//!     <img alt="Build Status" src="https://github.com/rune-rs/rune/workflows/Build/badge.svg">
//! </a>
//!
//! <a href="https://github.com/rune-rs/rune/actions">
//!     <img alt="Site Status" src="https://github.com/rune-rs/rune/workflows/Site/badge.svg">
//! </a>
//!
//! <a href="https://crates.io/crates/rune">
//!     <img alt="crates.io" src="https://img.shields.io/crates/v/rune.svg">
//! </a>
//!
//! <a href="https://docs.rs/rune">
//!     <img alt="docs.rs" src="https://docs.rs/rune/badge.svg">
//! </a>
//!
//! <a href="https://discord.gg/v5AeNkT">
//!     <img alt="Chat on Discord" src="https://img.shields.io/discord/558644981137670144.svg?logo=discord&style=flat-square">
//! </a>
//! </div>
//!
//! A cli for the [Rune Language].
//!
//! If you're in the repo, you can take it for a spin with:
//!
//! ```text
//! cargo run --bin rune -- scripts/hello_world.rn
//! ```
//!
//! [Rune Language]: https://rune-rs.github.io
//! [rune]: https://github.com/rune-rs/rune

use anyhow::Result;
use rune::compile::ParseOptionError;
use rune::termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use rune::{Context, ContextError, Options};
use rune_modules::capture_io::CaptureIo;
use std::error::Error;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use structopt::StructOpt;

mod benches;
mod check;
mod loader;
mod run;
mod tests;
mod visitor;

pub const VERSION: &str = include_str!(concat!(env!("OUT_DIR"), "/version.txt"));

#[derive(StructOpt, Debug, Clone)]
enum Command {
    /// Run checks but do not execute
    Check(check::Flags),
    /// Run all tests but do not execute
    Test(tests::Flags),
    /// Run the given program as a benchmark
    Bench(benches::Flags),
    /// Run the designated script
    Run(run::Flags),
}

impl Command {
    fn propagate_related_flags(&mut self) {
        match self {
            Command::Check(_) => {}
            Command::Test(args) => {
                args.shared.test = true;
            }
            Command::Bench(args) => {
                args.shared.test = true;
            }
            Command::Run(args) => {
                args.propagate_related_flags();
            }
        }
    }
}

#[derive(StructOpt, Debug, Clone)]
struct SharedFlags {
    /// Enable experimental features.
    ///
    /// This makes the `std::experimental` module available to scripts.
    #[structopt(long)]
    experimental: bool,

    /// Enabled the std::test experimental module.
    #[structopt(long)]
    test: bool,

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

    /// All paths to include in the command. By default, the tool searches the
    /// current directory and some known files for candidates.
    #[structopt(parse(from_os_str))]
    paths: Vec<PathBuf>,
}

impl SharedFlags {
    /// Construct a rune context according to the specified argument.
    fn context(&self) -> Result<Context, ContextError> {
        let mut context = rune_modules::default_context()?;

        if self.experimental {
            context.install(&rune_modules::experiments::module(true)?)?;
        }

        if self.test {
            context.install(&benches::test_module()?)?;
        }

        Ok(context)
    }

    /// Setup a context that captures output.
    fn context_with_capture(&self, io: &CaptureIo) -> Result<Context, ContextError> {
        let mut context = rune_modules::with_config(false)?;

        context.install(&rune_modules::capture_io::module(io)?)?;

        if self.experimental {
            context.install(&rune_modules::experiments::module(true)?)?;
        }

        if self.test {
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
            Command::Bench(_) | Command::Run(_) => (),
        }

        for option in &self.shared().compiler_options {
            options.parse_option(option)?;
        }

        Ok(options)
    }

    /// Access shared arguments.
    fn shared(&self) -> &SharedFlags {
        match &self.cmd {
            Command::Check(args) => &args.shared,
            Command::Test(args) => &args.shared,
            Command::Bench(args) => &args.shared,
            Command::Run(args) => &args.shared,
        }
    }

    /// Access shared arguments mutably.
    fn shared_mut(&mut self) -> &mut SharedFlags {
        match &mut self.cmd {
            Command::Check(args) => &mut args.shared,
            Command::Test(args) => &mut args.shared,
            Command::Bench(args) => &mut args.shared,
            Command::Run(args) => &mut args.shared,
        }
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

    let mut o = StandardStream::stdout(choice);
    env_logger::init();

    match main_with_out(&mut o, args).await {
        Ok(code) => Ok(code),
        Err(error) => {
            o.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
            let result = format_errors(&mut o, error.as_ref());
            o.set_color(&ColorSpec::new())?;
            let () = result?;
            Ok(ExitCode::Failure)
        }
    }
}

async fn main_with_out(o: &mut StandardStream, mut args: Args) -> Result<ExitCode> {
    args.cmd.propagate_related_flags();

    let shared = args.shared_mut();

    if shared.paths.is_empty() {
        for file in SPECIAL_FILES {
            let path = PathBuf::from(file);
            if path.exists() && path.is_file() {
                shared.paths.push(path);
                break;
            }
        }

        if shared.paths.is_empty() {
            writeln!(
                o,
                "Invalid usage: No input path given and no main or lib file found"
            )?;
            return Ok(ExitCode::Failure);
        }
    }

    let paths = loader::walk_paths(shared.recursive, std::mem::take(&mut shared.paths));

    let options = args.options()?;

    for path in paths {
        let path = path?;

        match run_path(o, &args, &options, &path).await? {
            ExitCode::Success => (),
            other => {
                return Ok(other);
            }
        }
    }

    Ok(ExitCode::Success)
}

/// Run a single path.
async fn run_path(
    o: &mut StandardStream,
    args: &Args,
    options: &Options,
    path: &Path,
) -> Result<ExitCode> {
    match &args.cmd {
        Command::Check(flags) => check::run(o, flags, options, path),
        Command::Test(flags) => {
            let io = rune_modules::capture_io::CaptureIo::new();
            let context = flags.shared.context_with_capture(&io)?;

            let load = loader::load(o, &context, args, options, path, visitor::Attribute::Test)?;

            tests::run(
                o,
                flags,
                &context,
                Some(&io),
                load.unit,
                &load.sources,
                &load.functions,
            )
            .await
        }
        Command::Bench(flags) => {
            let io = rune_modules::capture_io::CaptureIo::new();
            let context = flags.shared.context_with_capture(&io)?;

            let load = loader::load(o, &context, args, options, path, visitor::Attribute::Bench)?;

            benches::run(
                o,
                flags,
                &context,
                Some(&io),
                load.unit,
                &load.sources,
                &load.functions,
            )
            .await
        }
        Command::Run(flags) => {
            let context = flags.shared.context()?;

            let load = loader::load(o, &context, args, options, path, visitor::Attribute::None)?;

            run::run(o, flags, &context, load.unit, &load.sources).await
        }
    }
}
