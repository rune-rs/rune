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
//! [runestick]: https://github.com/rune-rs/rune

use anyhow::{Context as _, Result};
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{DumpInstructions as _, EmitDiagnostics as _, EmitSource as _};
use std::fs;
use std::io;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use structopt::StructOpt;

use runestick::{Unit, Value, VmExecution};
mod tests;

pub const VERSION: &str = include_str!(concat!(env!("OUT_DIR"), "/version.txt"));

type PathResult = Result<(
    Arc<Unit>,
    runestick::Context,
    Arc<runestick::RuntimeContext>,
    rune::Sources,
    Vec<(runestick::Hash, runestick::CompileMeta)>,
)>;

#[derive(StructOpt, Debug, Clone)]
enum Command {
    /// Run checks but do not execute
    Check(CheckFlags),

    /// Run all tests but do not execute
    Test(TestFlags),

    /// Run the designated script
    Run(RunFlags),
}

impl Command {
    fn propagate_related_flags(&mut self) {
        match self {
            Command::Check(_) => {}
            Command::Test(_) => {}
            Command::Run(args) => {
                if args.dump {
                    args.dump_unit = true;
                    args.dump_stack = true;
                    args.dump_functions = true;
                    args.dump_types = true;
                    args.dump_native_functions = true;
                    args.dump_native_types = true;
                }

                if args.dump_unit {
                    args.dump_unit = true;
                    args.dump_instructions = true;
                }

                if args.dump_functions
                    || args.dump_native_functions
                    || args.dump_stack
                    || args.dump_types
                    || args.dump_instructions
                {
                    args.dump_unit = true;
                }
            }
        }
    }
}

#[derive(StructOpt, Debug, Clone)]
struct SharedArgs {
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

    /// All paths to include in the command. By default, the tool searches the
    /// current directory and some known files for candidates.
    #[structopt(parse(from_os_str))]
    paths: Vec<PathBuf>,
}

impl SharedArgs {
    /// Construct a runestick context according to the specified argument.
    fn context(&self) -> Result<runestick::Context, runestick::ContextError> {
        let mut context = rune_modules::default_context()?;

        if self.experimental {
            context.install(&rune_modules::experiments::module(true)?)?;
        }

        Ok(context)
    }
}

#[derive(StructOpt, Debug, Clone)]
struct CheckFlags {
    /// Exit with a non-zero exit-code even for warnings
    #[structopt(long)]
    warnings_are_errors: bool,

    #[structopt(flatten)]
    shared: SharedArgs,
}

#[derive(StructOpt, Debug, Clone)]
pub(crate) struct TestFlags {
    /// Display one character per test instead of one line
    #[structopt(short = "q", long)]
    quiet: bool,

    /// Run all tests regardless of failure
    #[structopt(long)]
    no_fail_fast: bool,

    #[structopt(flatten)]
    shared: SharedArgs,
}

#[derive(StructOpt, Debug, Clone)]
struct RunFlags {
    /// Provide detailed tracing for each instruction executed.
    #[structopt(short, long)]
    trace: bool,
    /// Dump everything.
    #[structopt(short, long)]
    dump: bool,
    /// Dump default information about unit.
    #[structopt(long)]
    dump_unit: bool,
    /// Dump unit instructions.
    #[structopt(long)]
    dump_instructions: bool,
    /// Dump the state of the stack after completion.
    ///
    /// If compiled with `--trace` will dump it after each instruction.
    #[structopt(long)]
    dump_stack: bool,

    /// Dump dynamic functions.
    #[structopt(long)]
    dump_functions: bool,

    /// Dump dynamic types.
    #[structopt(long)]
    dump_types: bool,

    /// Dump native functions.
    #[structopt(long)]
    dump_native_functions: bool,

    /// Dump native types.
    #[structopt(long)]
    dump_native_types: bool,

    /// Include source code references where appropriate (only available if -O debug-info=true).
    #[structopt(long)]
    with_source: bool,

    #[structopt(flatten)]
    shared: SharedArgs,
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
    #[structopt(subcommand)] // Note that we mark a field as a subcommand
    cmd: Command,
}

impl Args {
    /// Construct compiler options from cli arguments.
    fn options(&self) -> Result<rune::Options, rune::ConfigurationError> {
        let mut options = rune::Options::default();

        // Command-specific override defaults.
        match &self.cmd {
            Command::Test(_) | Command::Check(_) => {
                options.test(true);
                options.bytecode(false);
            }
            Command::Run(_) => (),
        }

        for option in &self.shared().compiler_options {
            options.parse_option(option)?;
        }

        Ok(options)
    }

    /// Access shared arguments.
    fn shared(&self) -> &SharedArgs {
        match &self.cmd {
            Command::Check(args) => &args.shared,
            Command::Test(args) => &args.shared,
            Command::Run(args) => &args.shared,
        }
    }

    /// Access shared arguments mutably.
    fn shared_mut(&mut self) -> &mut SharedArgs {
        match &mut self.cmd {
            Command::Check(args) => &mut args.shared,
            Command::Test(args) => &mut args.shared,
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

async fn try_main() -> Result<ExitCode> {
    env_logger::init();

    let mut args = Args::from_args();
    args.cmd.propagate_related_flags();

    let options = args.options()?;

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
            println!("Invalid usage: No input path given and no main or lib file found");
            return Ok(ExitCode::Failure);
        }
    }

    let paths = walk_paths(shared.recursive, std::mem::take(&mut shared.paths));

    for path in paths {
        let path = path?;

        match run_path(&args, &options, &path).await? {
            ExitCode::Success => (),
            other => {
                return Ok(other);
            }
        }
    }

    Ok(ExitCode::Success)
}

fn walk_paths(recursive: bool, paths: Vec<PathBuf>) -> impl Iterator<Item = io::Result<PathBuf>> {
    use std::collections::VecDeque;
    use std::ffi::OsStr;

    let mut queue = paths.into_iter().collect::<VecDeque<_>>();

    std::iter::from_fn(move || loop {
        let path = queue.pop_front()?;

        if !recursive {
            return Some(Ok(path));
        }

        if path.is_file() {
            if path.extension() == Some(OsStr::new("rn")) {
                return Some(Ok(path));
            }

            continue;
        }

        let d = match fs::read_dir(path) {
            Ok(d) => d,
            Err(error) => return Some(Err(error)),
        };

        for e in d {
            let e = match e {
                Ok(e) => e,
                Err(error) => return Some(Err(error)),
            };

            queue.push_back(e.path());
        }
    })
}

/// Load context and code for a given path
fn load_path(
    out: &mut StandardStream,
    args: &Args,
    options: &rune::Options,
    path: &Path,
) -> PathResult {
    let shared = args.shared();
    let context = shared.context()?;

    let bytecode_path = path.with_extension("rnc");

    let source = runestick::Source::from_path(path)
        .with_context(|| format!("reading file: {}", path.display()))?;

    let runtime = Arc::new(context.runtime());
    let mut sources = rune::Sources::new();

    sources.insert(source);

    let use_cache = options.bytecode && should_cache_be_used(path, &bytecode_path)?;

    // TODO: how do we deal with tests discovery for bytecode loading
    let maybe_unit = if use_cache {
        let f = fs::File::open(&bytecode_path)?;

        match bincode::deserialize_from::<_, Unit>(f) {
            Ok(unit) => {
                log::trace!("using cache: {}", bytecode_path.display());
                Some(Arc::new(unit))
            }
            Err(e) => {
                log::error!("failed to deserialize: {}: {}", bytecode_path.display(), e);
                None
            }
        }
    } else {
        None
    };

    let (unit, tests) = match maybe_unit {
        Some(unit) => (unit, Default::default()),
        None => {
            log::trace!("building file: {}", path.display());

            let mut diagnostics = if shared.warnings {
                rune::Diagnostics::new()
            } else {
                rune::Diagnostics::without_warnings()
            };

            let test_finder = Rc::new(tests::TestVisitor::default());

            let result = rune::load_sources_with_visitor(
                &context,
                options,
                &mut sources,
                &mut diagnostics,
                test_finder.clone(),
                Rc::new(rune::FileSourceLoader::new()),
            );

            diagnostics.emit_diagnostics(out, &sources)?;
            let unit = result?;

            if options.bytecode {
                log::trace!("serializing cache: {}", bytecode_path.display());
                let f = fs::File::create(&bytecode_path)?;
                bincode::serialize_into(f, &unit)?;
            }

            let test_finder = match Rc::try_unwrap(test_finder) {
                Ok(test_finder) => test_finder,
                Err(..) => panic!("test finder should be uniquely held"),
            };

            (Arc::new(unit), test_finder.into_test_functions())
        }
    };

    Ok((unit, context, runtime, sources, tests))
}

/// Run a single path.
async fn run_path(args: &Args, options: &rune::Options, path: &Path) -> Result<ExitCode> {
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

    let mut out = StandardStream::stdout(choice);

    match &args.cmd {
        Command::Check(checkargs) => {
            writeln!(out, "Checking: {}", path.display())?;

            let context = checkargs.shared.context()?;

            let source = runestick::Source::from_path(path)
                .with_context(|| format!("reading file: {}", path.display()))?;

            let mut sources = rune::Sources::new();

            sources.insert(source);

            let mut diagnostics = if checkargs.shared.warnings || checkargs.warnings_are_errors {
                rune::Diagnostics::new()
            } else {
                rune::Diagnostics::without_warnings()
            };

            let _ = rune::load_sources_with_visitor(
                &context,
                options,
                &mut sources,
                &mut diagnostics,
                Rc::new(tests::TestVisitor::default()),
                Rc::new(rune::FileSourceLoader::new()),
            );

            diagnostics.emit_diagnostics(&mut out, &sources).unwrap();

            if diagnostics.has_error()
                || (checkargs.warnings_are_errors && diagnostics.has_warning())
            {
                Ok(ExitCode::Failure)
            } else {
                Ok(ExitCode::Success)
            }
        }
        Command::Test(testflags) => match load_path(&mut out, args, options, path) {
            Ok((unit, _context, runtime, sources, tests)) => {
                tests::do_tests(testflags, out, runtime, unit, sources, tests).await
            }
            Err(_) => Ok(ExitCode::Failure),
        },
        Command::Run(runargs) => {
            let (unit, context, runtime, sources, _tests) =
                match load_path(&mut out, args, options, path) {
                    Ok(v) => v,
                    Err(_) => return Ok(ExitCode::Failure),
                };

            if runargs.dump_native_functions {
                writeln!(out, "# functions")?;

                for (i, (hash, f)) in context.iter_functions().enumerate() {
                    writeln!(out, "{:04} = {} ({})", i, f, hash)?;
                }
            }

            if runargs.dump_native_types {
                writeln!(out, "# types")?;

                for (i, (hash, ty)) in context.iter_types().enumerate() {
                    writeln!(out, "{:04} = {} ({})", i, ty, hash)?;
                }
            }

            if runargs.dump_unit {
                if runargs.dump_instructions {
                    writeln!(out, "# instructions")?;
                    let mut out = out.lock();
                    unit.dump_instructions(&mut out, &sources, runargs.with_source)?;
                }

                let mut functions = unit.iter_functions().peekable();
                let mut strings = unit.iter_static_strings().peekable();
                let mut keys = unit.iter_static_object_keys().peekable();

                if runargs.dump_functions && functions.peek().is_some() {
                    writeln!(out, "# dynamic functions")?;

                    for (hash, kind) in functions {
                        if let Some(signature) =
                            unit.debug_info().and_then(|d| d.functions.get(&hash))
                        {
                            writeln!(out, "{} = {}", hash, signature)?;
                        } else {
                            writeln!(out, "{} = {}", hash, kind)?;
                        }
                    }
                }

                if strings.peek().is_some() {
                    writeln!(out, "# strings")?;

                    for string in strings {
                        writeln!(out, "{} = {:?}", string.hash(), string)?;
                    }
                }

                if keys.peek().is_some() {
                    writeln!(out, "# object keys")?;

                    for (hash, keys) in keys {
                        writeln!(out, "{} = {:?}", hash, keys)?;
                    }
                }
            }
            do_run(runargs, out, runtime, unit, sources).await
        }
    }
}

async fn do_run(
    args: &RunFlags,
    mut out: StandardStream,
    runtime: Arc<runestick::RuntimeContext>,
    unit: Arc<Unit>,
    sources: rune::Sources,
) -> Result<ExitCode> {
    let last = std::time::Instant::now();

    let vm = runestick::Vm::new(runtime, unit.clone());
    let mut execution: runestick::VmExecution = vm.execute(&["main"], ())?;
    let result = if args.trace {
        match do_trace(
            &mut out,
            &mut execution,
            &sources,
            args.dump_stack,
            args.with_source,
        )
        .await
        {
            Ok(value) => Ok(value),
            Err(TraceError::Io(io)) => return Err(io.into()),
            Err(TraceError::VmError(vm)) => Err(vm),
        }
    } else {
        execution.async_complete().await
    };

    let errored;

    match result {
        Ok(result) => {
            let duration = std::time::Instant::now().duration_since(last);
            writeln!(out, "== {:?} ({:?})", result, duration)?;
            errored = None;
        }
        Err(error) => {
            let duration = std::time::Instant::now().duration_since(last);
            writeln!(out, "== ! ({}) ({:?})", error, duration)?;
            errored = Some(error);
        }
    };

    if args.dump_stack {
        writeln!(out, "# full stack dump after halting")?;

        let vm = execution.vm()?;

        let frames = vm.call_frames();
        let stack = vm.stack();

        let mut it = frames.iter().enumerate().peekable();

        while let Some((count, frame)) = it.next() {
            let stack_top = match it.peek() {
                Some((_, next)) => next.stack_bottom(),
                None => stack.stack_bottom(),
            };

            let values = stack
                .get(frame.stack_bottom()..stack_top)
                .expect("bad stack slice");

            writeln!(out, "  frame #{} (+{})", count, frame.stack_bottom())?;

            if values.is_empty() {
                writeln!(out, "    *empty*")?;
            }

            for (n, value) in stack.iter().enumerate() {
                writeln!(out, "{}+{} = {:?}", frame.stack_bottom(), n, value)?;
            }
        }

        // NB: print final frame
        writeln!(out, "  frame #{} (+{})", frames.len(), stack.stack_bottom())?;

        let values = stack.get(stack.stack_bottom()..).expect("bad stack slice");

        if values.is_empty() {
            writeln!(out, "    *empty*")?;
        }

        for (n, value) in values.iter().enumerate() {
            writeln!(out, "    {}+{} = {:?}", stack.stack_bottom(), n, value)?;
        }
    }

    if let Some(error) = errored {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        error.emit_diagnostics(&mut writer, &sources)?;
        Ok(ExitCode::VmError)
    } else {
        Ok(ExitCode::Success)
    }
}

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
            eprintln!("Error: {}", error);
            std::process::exit(-1);
        }
    }
}

enum TraceError {
    Io(std::io::Error),
    VmError(runestick::VmError),
}

impl From<std::io::Error> for TraceError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// Perform a detailed trace of the program.
async fn do_trace(
    out: &mut StandardStream,
    execution: &mut VmExecution,
    sources: &rune::Sources,
    dump_stack: bool,
    with_source: bool,
) -> Result<Value, TraceError> {
    let mut current_frame_len = execution
        .vm()
        .map_err(TraceError::VmError)?
        .call_frames()
        .len();

    loop {
        {
            let vm = execution.vm().map_err(TraceError::VmError)?;
            let mut out = out.lock();

            if let Some((hash, signature)) =
                vm.unit().debug_info().and_then(|d| d.function_at(vm.ip()))
            {
                writeln!(out, "fn {} ({}):", signature, hash)?;
            }

            let debug = vm
                .unit()
                .debug_info()
                .and_then(|d| d.instruction_at(vm.ip()));

            if with_source {
                let debug_info = debug.and_then(|d| sources.get(d.source_id).map(|s| (s, d.span)));
                if let Some((source, span)) = debug_info {
                    source.emit_source_line(&mut out, span)?;
                }
            }

            if let Some(label) = debug.and_then(|d| d.label.as_ref()) {
                writeln!(out, "{}:", label)?;
            }

            if let Some(inst) = vm.unit().instruction_at(vm.ip()) {
                write!(out, "  {:04} = {}", vm.ip(), inst)?;
            } else {
                write!(out, "  {:04} = *out of bounds*", vm.ip())?;
            }

            if let Some(comment) = debug.and_then(|d| d.comment.as_ref()) {
                write!(out, " // {}", comment)?;
            }

            writeln!(out)?;
        }

        let result = match execution.async_step().await {
            Ok(result) => result,
            Err(e) => return Err(TraceError::VmError(e)),
        };

        let mut out = out.lock();

        if dump_stack {
            let vm = execution.vm().map_err(TraceError::VmError)?;
            let frames = vm.call_frames();

            let stack = vm.stack();

            if current_frame_len != frames.len() {
                if current_frame_len < frames.len() {
                    writeln!(out, "=> frame {} ({}):", frames.len(), stack.stack_bottom())?;
                } else {
                    writeln!(out, "<= frame {} ({}):", frames.len(), stack.stack_bottom())?;
                }

                current_frame_len = frames.len();
            }

            let values = stack.get(stack.stack_bottom()..).expect("bad stack slice");

            if values.is_empty() {
                writeln!(out, "    *empty*")?;
            }

            for (n, value) in values.iter().enumerate() {
                writeln!(out, "    {}+{} = {:?}", stack.stack_bottom(), n, value)?;
            }
        }

        if let Some(result) = result {
            break Ok(result);
        }
    }
}

/// Test if path `a` is newer than path `b`.
fn should_cache_be_used(source: &Path, cached: &Path) -> io::Result<bool> {
    let source = fs::metadata(source)?;

    let cached = match fs::metadata(cached) {
        Ok(cached) => cached,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };

    Ok(source.modified()? < cached.modified()?)
}
