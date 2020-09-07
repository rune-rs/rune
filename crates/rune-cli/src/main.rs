//! <div align="center">
//! <a href="https://rune-rs.github.io/rune/">
//!     <b>Read the Book 📖</b>
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
//!     <img alt="Book Status" src="https://github.com/rune-rs/rune/workflows/Book/badge.svg">
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
//! cargo run -- scripts/hello_world.rn
//! ```
//!
//! [Rune Language]: https://github.com/rune-rs/rune
//! [runestick]: https://github.com/rune-rs/rune

use anyhow::{bail, Result};
use rune::termcolor::{ColorChoice, StandardStream};
use rune::EmitDiagnostics as _;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;

use runestick::{Item, Value, VmExecution};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let mut args = env::args();
    args.next();

    let mut path = None;
    let mut trace = false;
    let mut dump_unit = false;
    let mut dump_stack = false;
    let mut dump_functions = false;
    let mut dump_types = false;
    let mut help = false;

    let mut options = rune::Options::default();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--" => continue,
            "--trace" => {
                trace = true;
            }
            "--dump" => {
                dump_unit = true;
                dump_stack = true;
                dump_functions = true;
                dump_types = true;
            }
            "--dump-unit" => {
                dump_unit = true;
            }
            "--dump-stack" => {
                dump_stack = true;
            }
            "--dump-functions" => {
                dump_functions = true;
            }
            "--dump-types" => {
                dump_types = true;
            }
            "-O" => {
                let opt = match args.next() {
                    Some(opt) => opt,
                    None => {
                        println!("expected optimization option to `-O`");
                        return Ok(());
                    }
                };

                options.parse_option(&opt)?;
            }
            "--help" | "-h" => {
                help = true;
            }
            other if !other.starts_with('-') => {
                path = Some(PathBuf::from(other));
            }
            other => {
                println!("Unrecognized option: {}", other);
                help = true;
            }
        }
    }

    const USAGE: &str = "rune-cli [--trace] <file>";

    if help {
        println!("Usage: {}", USAGE);
        println!();
        println!("  --help, -h         - Show this help.");
        println!("  --trace           - Provide detailed tracing for each instruction executed.");
        println!("  --dump            - Dump all forms of diagnostic.");
        println!("  --dump-unit       - Dump diagnostics on the unit generated from the file.");
        println!("  --dump-stack      - Dump the state of the stack after completion. If compiled with `--trace` will dump it after each instruction.");
        println!("  --dump-functions  - Dump available functions.");
        println!("  --dump-types      - Dump available types.");
        println!("  --no-linking      - Disable link time checks.");
        println!();
        println!("Compiler options:");
        println!("  -O <option>       - Update the given compiler option.");
        println!();
        println!("Available <option> arguments:");
        println!("  memoize-instance-fn[=<true/false>] - Inline the lookup of an instance function where appropriate.");
        println!("  link-checks[=<true/false>] - Perform linker checks which makes sure that called functions exist.");
        return Ok(());
    }

    let path = match path {
        Some(path) => path,
        None => {
            bail!("Invalid usage: {}", USAGE);
        }
    };

    let mut context = rune::default_context()?;
    let context = Arc::new(context);

    let mut warnings = rune::Warnings::new();
    let mut sources = rune::Sources::new();

    let unit = match rune::load_path(&*context, &options, &mut sources, &path, &mut warnings) {
        Ok(unit) => Arc::new(unit),
        Err(error) => {
            let mut writer = StandardStream::stderr(ColorChoice::Always);
            error.emit_diagnostics(&mut writer, &sources)?;
            return Ok(());
        }
    };

    let vm = runestick::Vm::new(context.clone(), unit.clone());

    if !warnings.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        warnings.emit_diagnostics(&mut writer, &sources)?;
    }

    if dump_functions {
        println!("# functions");

        for (i, (hash, f)) in context.iter_functions().enumerate() {
            println!("{:04} = {} ({})", i, f, hash);
        }
    }

    if dump_types {
        println!("# types");

        for (i, (hash, ty)) in context.iter_types().enumerate() {
            println!("{:04} = {} ({})", i, ty, hash);
        }
    }

    if dump_unit {
        use std::io::Write as _;

        println!("# instructions:");

        let mut first_function = true;

        for (n, inst) in vm.unit().iter_instructions().enumerate() {
            let out = std::io::stdout();
            let mut out = out.lock();

            let debug = vm.unit().debug_info().and_then(|d| d.instruction_at(n));

            if let Some((hash, signature)) = vm.unit().debug_info().and_then(|d| d.function_at(n)) {
                if first_function {
                    first_function = false;
                } else {
                    println!();
                }

                println!("fn {} ({}):", signature, hash);
            }

            if let Some(label) = debug.and_then(|d| d.label.as_ref()) {
                println!("{}:", label);
            }

            write!(out, "  {:04} = {}", n, inst)?;

            if let Some(comment) = debug.and_then(|d| d.comment.as_ref()) {
                write!(out, " // {}", comment)?;
            }

            println!();
        }

        println!("# functions:");

        for (hash, kind) in vm.unit().iter_functions() {
            if let Some(signature) = vm.unit().debug_info().and_then(|d| d.functions.get(&hash)) {
                println!("{} = {}", hash, signature);
            } else {
                println!("{} = {}", hash, kind);
            }
        }

        println!("# strings:");

        for string in vm.unit().iter_static_strings() {
            println!("{} = {:?}", string.hash(), string);
        }

        println!("# object keys:");

        for (hash, keys) in vm.unit().iter_static_object_keys() {
            println!("{} = {:?}", hash, keys);
        }

        println!("---");
    }

    let mut execution: runestick::VmExecution = vm.call(Item::of(&["main"]), ())?;
    let last = std::time::Instant::now();

    let result = if trace {
        match do_trace(&mut execution, dump_stack).await {
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
            println!("== {:?} ({:?})", result, duration);
            errored = None;
        }
        Err(error) => {
            let duration = std::time::Instant::now().duration_since(last);
            println!("== ! ({}) ({:?})", error, duration);
            errored = Some(error);
        }
    };

    if dump_stack {
        println!("# full stack dump after halting");

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
            println!("  frame #{} (+{})", count, frame.stack_bottom());

            if values.is_empty() {
                println!("    *empty*");
            }

            for (n, value) in stack.iter().enumerate() {
                println!("{}+{} = {:?}", frame.stack_bottom(), n, value);
            }
        }

        // NB: print final frame
        println!("  frame #{} (+{})", frames.len(), stack.stack_bottom());

        let values = stack.get(stack.stack_bottom()..).expect("bad stack slice");

        if values.is_empty() {
            println!("    *empty*");
        }

        for (n, value) in values.iter().enumerate() {
            println!("    {}+{} = {:?}", stack.stack_bottom(), n, value);
        }
    }

    if let Some(error) = errored {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        error.emit_diagnostics(&mut writer, &sources)?;
    }

    Ok(())
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
async fn do_trace(execution: &mut VmExecution, dump_stack: bool) -> Result<Value, TraceError> {
    use std::io::Write as _;
    let out = std::io::stdout();

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

            let debug_inst = vm
                .unit()
                .debug_info()
                .and_then(|debug| debug.instruction_at(vm.ip()));

            if let Some(inst) = debug_inst {
                if let Some(label) = inst.label {
                    writeln!(out, "{}:", label)?;
                }
            }

            if let Some(inst) = vm.unit().instruction_at(vm.ip()) {
                write!(out, "  {:04} = {}", vm.ip(), inst)?;
            } else {
                write!(out, "  {:04} = *out of bounds*", vm.ip())?;
            }

            if let Some(inst) = debug_inst {
                if let Some(comment) = &inst.comment {
                    write!(out, " // {}", comment)?;
                }
            }

            writeln!(out,)?;
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
                    println!("=> frame {} ({}):", frames.len(), stack.stack_bottom());
                } else {
                    println!("<= frame {} ({}):", frames.len(), stack.stack_bottom());
                }

                current_frame_len = frames.len();
            }

            let values = stack.get(stack.stack_bottom()..).expect("bad stack slice");

            if values.is_empty() {
                println!("    *empty*");
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
