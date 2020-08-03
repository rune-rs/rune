use anyhow::{bail, Result};
use std::env;
use std::error::Error;
use std::io::Write as _;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let mut args = env::args();
    args.next();

    let mut path = None;
    let mut trace = false;
    let mut dump_unit = false;
    let mut dump_vm = false;
    let mut dump_functions = false;
    let mut dump_types = false;
    let mut help = false;

    let mut context = st::Context::with_default_packages()?;
    context.install(st_http::module()?)?;
    context.install(st_json::module()?)?;

    let mut runtime = rune::Runtime::with_context(context);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--" => continue,
            "--trace" => {
                trace = true;
            }
            "--dump" => {
                dump_unit = true;
                dump_vm = true;
                dump_functions = true;
                dump_types = true;
            }
            "--dump-unit" => {
                dump_unit = true;
            }
            "--dump-vm" => {
                dump_vm = true;
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

                runtime.parse_optimization(&[&opt])?;
            }
            "--help" | "-h" => {
                help = true;
            }
            other if !other.starts_with("-") => {
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
        println!("  --dump-vm         - Dump diagnostics on VM state. If combined with `--trace`, does so afte each instruction.");
        println!("  --dump-functions  - Dump available functions.");
        println!("  --dump-types      - Dump available types.");
        println!();
        println!("Compiler options:");
        println!("  -O <optimization> - Update the given optimization option.");
        println!();
        println!("Available <optimization> arguments:");
        println!("  memoize-instance-fn[=<true/false>] - Inline the lookup of an instance function where appropriate.");
        return Ok(());
    }

    let path = match path {
        Some(path) => PathBuf::from(path),
        None => {
            bail!("Invalid usage: {}", USAGE);
        }
    };

    let file_id = match runtime.load(&path) {
        Ok(file_id) => file_id,
        Err(e) => {
            use rune::termcolor;
            let mut writer = termcolor::StandardStream::stderr(termcolor::ColorChoice::Always);
            writeln!(writer, "failed to load: {}: {}", path.display(), e)?;
            runtime.emit_diagnostics(&mut writer)?;
            return Ok(());
        }
    };

    if dump_functions {
        println!("# functions");

        for (i, (hash, f)) in runtime.context().iter_functions().enumerate() {
            println!("{:04} = {} ({})", i, f, hash);
        }
    }

    if dump_types {
        println!("# types");

        for (i, (hash, ty)) in runtime.context().iter_types().enumerate() {
            println!("{:04} = {} ({})", i, ty, hash);
        }
    }

    if dump_unit {
        use std::io::Write as _;

        let unit = match runtime.unit(file_id) {
            Some(unit) => unit,
            None => bail!("missing unit"),
        };

        println!("# instructions:");

        let mut first_function = true;

        for (n, inst) in unit.iter_instructions().enumerate() {
            let out = std::io::stdout();
            let mut out = out.lock();

            let debug = unit.debug_info_at(n);

            if let Some((hash, function)) = unit.function_at(n) {
                if first_function {
                    first_function = false;
                } else {
                    println!();
                }

                println!("fn {} ({}):", function.signature, hash);
            }

            if let Some(debug) = debug {
                if let Some(label) = debug.label {
                    println!("{}:", label);
                }
            }

            write!(out, "  {:04} = {}", n, inst)?;

            if let Some(debug) = debug {
                if let Some(comment) = &debug.comment {
                    write!(out, " // {}", comment)?;
                }
            }

            println!();
        }

        println!("# imports:");

        for (hash, f) in unit.iter_imports() {
            println!("{} = {}", hash, f);
        }

        println!("# functions:");

        for (hash, f) in unit.iter_functions() {
            println!("{} = {} (at: {})", hash, f.signature, f.offset);
        }

        println!("# strings:");

        for (hash, string) in unit.iter_static_strings() {
            println!("{} = {:?}", hash, string);
        }

        println!("# object keys:");

        for (hash, keys) in unit.iter_static_object_keys() {
            println!("{} = {:?}", hash, keys);
        }

        println!("---");
    }

    let task: st::Task<st::Value> = runtime.call_function(file_id, &["main"], ())?;
    let last = std::time::Instant::now();

    let result = if trace {
        match do_trace(task, dump_vm).await {
            Ok(value) => Ok(value),
            Err(TraceError::Io(io)) => return Err(io.into()),
            Err(TraceError::VmError(vm)) => Err(vm),
        }
    } else {
        task.run_to_completion().await
    };

    let result = match result {
        Ok(result) => result,
        Err(e) => {
            // NB: this only works if we have debuginfo.
            match runtime.register_vm_error(file_id, e) {
                Ok(()) => {
                    use rune::termcolor;
                    let mut writer =
                        termcolor::StandardStream::stderr(termcolor::ColorChoice::Always);
                    runtime.emit_diagnostics(&mut writer)?;
                }
                Err(e) => {
                    println!("#0: {}", e);

                    let mut e = &e as &dyn Error;
                    let mut i = 1;

                    while let Some(err) = e.source() {
                        println!("#{}: {}", i, err);
                        i += 1;
                        e = err;
                    }
                }
            }

            return Ok(());
        }
    };

    let duration = std::time::Instant::now().duration_since(last);
    println!("== {:?} ({:?})", result, duration);

    if dump_vm {
        println!("# stack dump after completion");

        for (n, (_, value)) in runtime.vm().iter_stack_debug().enumerate() {
            println!("{} = {:?}", n, value);
        }

        println!("---");
    }

    Ok(())
}

enum TraceError {
    Io(std::io::Error),
    VmError(st::VmError),
}

impl From<std::io::Error> for TraceError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<st::VmError> for TraceError {
    fn from(error: st::VmError) -> Self {
        Self::VmError(error)
    }
}

/// Perform a detailed trace of the program.
async fn do_trace<T>(mut task: st::Task<'_, T>, dump_vm: bool) -> Result<T, TraceError>
where
    T: st::FromValue,
{
    use std::io::Write as _;
    let out = std::io::stdout();

    loop {
        {
            let mut out = out.lock();

            let debug = task.unit.debug_info_at(task.vm.ip());

            if let Some((hash, function)) = task.unit.function_at(task.vm.ip()) {
                writeln!(out, "fn {} ({}):", function.signature, hash)?;
            }

            if let Some(debug) = debug {
                if let Some(label) = debug.label {
                    writeln!(out, "{}:", label)?;
                }
            }

            if let Some(inst) = task.unit.instruction_at(task.vm.ip()) {
                write!(out, "  {:04} = {}", task.vm.ip(), inst)?;
            } else {
                write!(out, "  {:04} = *out of bounds*", task.vm.ip(),)?;
            }

            if let Some(debug) = debug {
                if let Some(comment) = &debug.comment {
                    write!(out, " // {}", comment)?;
                }
            }

            writeln!(out,)?;
        }

        let result = task.step().await?;

        let mut out = out.lock();

        if dump_vm {
            writeln!(out, "# stack dump")?;

            for (n, (_, value)) in task.vm.iter_stack_debug().enumerate() {
                writeln!(out, "{} = {:?}", n, value)?;
            }

            writeln!(out, "---")?;
        }

        if let Some(result) = result {
            break Ok(result);
        }
    }
}
