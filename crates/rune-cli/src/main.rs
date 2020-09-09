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
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use runestick::{Item, Unit, Value, VmExecution};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let mut args = env::args();
    args.next();

    let mut path = None;
    let mut trace = false;
    let mut dump_unit = false;
    let mut dump_instructions = false;
    let mut dump_stack = false;
    let mut dump_functions = false;
    let mut dump_types = false;
    let mut dump_native_functions = false;
    let mut dump_native_types = false;
    let mut with_source = false;
    let mut help = false;
    let mut experimental = false;

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
                dump_native_functions = true;
                dump_native_types = true;
            }
            "--dump-unit" => {
                dump_unit = true;
                dump_instructions = true;
            }
            "--dump-stack" => {
                dump_stack = true;
            }
            "--dump-instructions" => {
                dump_unit = true;
                dump_instructions = true;
            }
            "--dump-functions" => {
                dump_unit = true;
                dump_functions = true;
            }
            "--dump-types" => {
                dump_unit = true;
                dump_types = true;
            }
            "--dump-native-functions" => {
                dump_native_functions = true;
            }
            "--dump-native-types" => {
                dump_native_types = true;
            }
            "--with-source" => {
                with_source = true;
            }
            "--experimental" => {
                experimental = true;
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
        println!("  --help, -h               - Show this help.");
        println!(
            "  --trace                  - Provide detailed tracing for each instruction executed."
        );
        println!("  --dump                   - Dump everything.");
        println!("  --dump-unit              - Dump default information about unit.");
        println!("  --dump-instructions      - Dump unit instructions.");
        println!("  --dump-stack             - Dump the state of the stack after completion. If compiled with `--trace` will dump it after each instruction.");
        println!("  --dump-functions         - Dump dynamic functions.");
        println!("  --dump-types             - Dump dynamic types.");
        println!("  --dump-native-functions  - Dump native functions.");
        println!("  --dump-native-types      - Dump native types.");
        println!("  --with-source            - Include source code references where appropriate (only available if -O debug-info=true).");
        println!("  --experimental           - Enabled experimental features.");
        println!();
        println!("Compiler options:");
        println!("  -O <option>       - Update the given compiler option.");
        println!();
        println!("Available <option> arguments:");
        println!("  memoize-instance-fn[=<true/false>] - Inline the lookup of an instance function where appropriate.");
        println!("  link-checks[=<true/false>]         - Perform linker checks which makes sure that called functions exist.");
        println!("  debug-info[=<true/false>]          - Enable or disable debug info.");
        println!("  macros[=<true/false>]              - Enable or disable macros (experimental).");
        println!("  bytecode[=<true/false>]            - Enable or disable bytecode caching (experimental).");
        return Ok(());
    }

    let path = match path {
        Some(path) => path,
        None => {
            bail!("Invalid usage: {}", USAGE);
        }
    };

    let bytecode_path = path.with_extension("rnc");
    let mut context = rune::default_context()?;

    if experimental {
        context.install(&rune_macros::module()?)?;
    }

    let context = Arc::new(context);
    let mut sources = rune::Sources::new();
    let mut warnings = rune::Warnings::new();

    let use_cache = options.bytecode && should_cache_be_used(&path, &bytecode_path)?;
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

    let unit = match maybe_unit {
        Some(unit) => unit,
        None => {
            log::trace!("building file: {}", path.display());

            let unit =
                match rune::load_path(&*context, &options, &mut sources, &path, &mut warnings) {
                    Ok(unit) => unit,
                    Err(error) => {
                        let mut writer = StandardStream::stderr(ColorChoice::Always);
                        error.emit_diagnostics(&mut writer, &sources)?;
                        return Ok(());
                    }
                };

            if options.bytecode {
                log::trace!("serializing cache: {}", bytecode_path.display());
                let f = fs::File::create(&bytecode_path)?;
                bincode::serialize_into(f, &unit)?;
            }

            Arc::new(unit)
        }
    };

    if !warnings.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        warnings.emit_diagnostics(&mut writer, &sources)?;
    }

    let vm = runestick::Vm::new(context.clone(), unit.clone());

    if dump_native_functions {
        println!("# functions");

        for (i, (hash, f)) in context.iter_functions().enumerate() {
            println!("{:04} = {} ({})", i, f, hash);
        }
    }

    if dump_native_types {
        println!("# types");

        for (i, (hash, ty)) in context.iter_types().enumerate() {
            println!("{:04} = {} ({})", i, ty, hash);
        }
    }

    if dump_unit {
        use std::io::Write as _;

        let unit = vm.unit();

        if dump_instructions {
            println!("# instructions");

            let mut first_function = true;

            for (n, inst) in unit.iter_instructions().enumerate() {
                let out = std::io::stdout();
                let mut out = out.lock();

                let debug = unit.debug_info().and_then(|d| d.instruction_at(n));

                if let Some((hash, signature)) = unit.debug_info().and_then(|d| d.function_at(n)) {
                    if first_function {
                        first_function = false;
                    } else {
                        println!();
                    }

                    println!("fn {} ({}):", signature, hash);
                }

                if with_source {
                    if let Some((source, span)) =
                        debug.and_then(|d| sources.get(d.source_id).map(|s| (s, d.span)))
                    {
                        if let Some((count, line)) =
                            rune::diagnostics::line_for(source.as_str(), span)
                        {
                            writeln!(
                                out,
                                "  {}:{: <3} - {}",
                                source.name(),
                                count + 1,
                                line.trim_end()
                            )?;
                        }
                    }
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
        }

        let mut functions = unit.iter_functions().peekable();
        let mut types = unit.iter_types().peekable();
        let mut strings = unit.iter_static_strings().peekable();
        let mut keys = unit.iter_static_object_keys().peekable();

        if dump_functions && functions.peek().is_some() {
            println!("# dynamic functions");

            for (hash, kind) in functions {
                if let Some(signature) = unit.debug_info().and_then(|d| d.functions.get(&hash)) {
                    println!("{} = {}", hash, signature);
                } else {
                    println!("{} = {}", hash, kind);
                }
            }
        }

        if dump_types && types.peek().is_some() {
            println!("# dynamic types");

            for (hash, ty) in types {
                println!("{} = {}", hash, ty.value_type);
            }
        }

        if strings.peek().is_some() {
            println!("# strings");

            for string in strings {
                println!("{} = {:?}", string.hash(), string);
            }
        }

        if keys.peek().is_some() {
            println!("# object keys");

            for (hash, keys) in keys {
                println!("{} = {:?}", hash, keys);
            }
        }
    }

    let last = std::time::Instant::now();

    let mut execution: runestick::VmExecution = vm.call(&Item::of(&["main"]), ())?;

    let result = if trace {
        match do_trace(&mut execution, &sources, dump_stack, with_source).await {
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
async fn do_trace(
    execution: &mut VmExecution,
    sources: &rune::Sources,
    dump_stack: bool,
    with_source: bool,
) -> Result<Value, TraceError> {
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

            let debug = vm
                .unit()
                .debug_info()
                .and_then(|d| d.instruction_at(vm.ip()));

            if with_source {
                if let Some((source, span)) =
                    debug.and_then(|d| sources.get(d.source_id).map(|s| (s, d.span)))
                {
                    if let Some((count, line)) = rune::diagnostics::line_for(source.as_str(), span)
                    {
                        writeln!(
                            out,
                            "  {}:{: <3} - {}",
                            source.name(),
                            count + 1,
                            line.trim_end()
                        )?;
                    }
                }
            }

            if let Some(inst) = debug {
                if let Some(label) = &inst.label {
                    writeln!(out, "{}:", label)?;
                }
            }

            if let Some(inst) = vm.unit().instruction_at(vm.ip()) {
                write!(out, "  {:04} = {}", vm.ip(), inst)?;
            } else {
                write!(out, "  {:04} = *out of bounds*", vm.ip())?;
            }

            if let Some(inst) = debug {
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
