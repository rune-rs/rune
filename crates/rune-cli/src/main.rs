use anyhow::{bail, Result};
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use rune::SpannedError as _;

fn compile(source: &str, options: rune::Options) -> rune::Result<st::Unit> {
    let unit = rune::parse_all::<rune::ast::File>(&source)?;
    Ok(unit.compile_with_options(options)?)
}

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

    let mut options = rune::Options::default();

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

                options.optimizations.parse_option(&opt)?;
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

    let source = fs::read_to_string(&path)?;

    let unit = match compile(&source, options) {
        Ok(unit) => unit,
        Err(e) => {
            emit_diagnostics("compile error", &path, &source, e.span(), &e)?;
            return Ok(());
        }
    };

    let mut context = st::Context::with_default_packages()?;
    context.install(st_http::module()?)?;
    context.install(st_json::module()?)?;

    let mut errors = st::unit::LinkerErrors::new();

    if !unit.link(&context, &mut errors) {
        emit_link_diagnostics(&path, &source, errors)?;
        return Ok(());
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

        for (n, inst) in unit.iter_instructions().enumerate() {
            let out = std::io::stdout();
            let mut out = out.lock();

            let debug = unit.debug_info_at(n);

            if let Some((hash, function)) = unit.function_at(n) {
                if first_function {
                    first_function = false;
                } else {
                    writeln!(out)?;
                }

                writeln!(out, "fn {} ({}):", function.signature, hash)?;
            }

            if let Some(debug) = debug {
                if let Some(label) = debug.label {
                    writeln!(out, "{}:", label)?;
                }
            }

            write!(out, "  {:04} = {}", n, inst)?;

            if let Some(debug) = debug {
                if let Some(comment) = &debug.comment {
                    write!(out, " // {}", comment)?;
                }
            }

            writeln!(out)?;
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

        println!("---");
    }

    let mut vm = st::Vm::new();

    let task: st::Task<st::Value> = vm.call_function(&context, &unit, &["main"], ())?;
    let last = std::time::Instant::now();

    let result = if trace {
        do_trace(task, dump_vm).await
    } else {
        task.run_to_completion().await.map_err(anyhow::Error::from)
    };

    let result = match result {
        Ok(result) => result,
        Err(e) => {
            if let Some(debug) = unit.debug_info_at(vm.ip()) {
                emit_diagnostics("runtime error", &path, &source, debug.span, &*e)?;
                return Ok(());
            }

            println!("warning: debuginfo not available!");
            println!("#0: {}", e);

            let mut e = &*e as &dyn Error;
            let mut i = 1;

            while let Some(err) = e.source() {
                println!("#{}: {}", i, err);
                i += 1;
                e = err;
            }

            return Ok(());
        }
    };

    let duration = std::time::Instant::now().duration_since(last);
    println!("== {:?} ({:?})", result, duration);

    if dump_vm {
        println!("# stack dump after completion");

        for (n, (slot, value)) in vm.iter_stack_debug().enumerate() {
            if let st::ValuePtr::Managed(..) = slot {
                println!("{} = {:?} => {:?}", n, slot, value);
            } else {
                println!("{} = {:?}", n, slot);
            }
        }

        println!("---");
    }

    Ok(())
}

/// Perform a detailed trace of the program.
async fn do_trace<T>(mut task: st::Task<'_, T>, dump_vm: bool) -> Result<T>
where
    T: st::FromValue,
{
    loop {
        use std::io::Write as _;

        let out = std::io::stdout();
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

        writeln!(out)?;

        let result = task.step().await?;

        if dump_vm {
            println!("# stack dump");

            for (n, (slot, value)) in task.vm.iter_stack_debug().enumerate() {
                if let st::ValuePtr::Managed(..) = slot {
                    println!("{} = {:?} => {:?}", n, slot, value);
                } else {
                    println!("{} = {:?}", n, slot);
                }
            }

            println!("---");
        }

        if let Some(result) = result {
            break Ok(result);
        }
    }
}

fn emit_link_diagnostics(path: &Path, source: &str, errors: st::unit::LinkerErrors) -> Result<()> {
    use codespan_reporting::diagnostic::{Diagnostic, Label};
    use codespan_reporting::files::SimpleFiles;
    use codespan_reporting::term;
    use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};

    let mut files = SimpleFiles::new();

    let source_file = files.add(path.display().to_string(), source);

    let writer = StandardStream::stderr(ColorChoice::Always);
    let config = codespan_reporting::term::Config::default();

    for error in errors.errors() {
        match error {
            st::unit::LinkerError::MissingFunction { hash, spans } => {
                let mut labels = Vec::new();

                for span in spans {
                    labels.push(
                        Label::primary(source_file, span.start..span.end)
                            .with_message("called here."),
                    );
                }

                let diagnostic = Diagnostic::error()
                    .with_message(format!("missing function with hash `{}`", hash))
                    .with_labels(labels);

                term::emit(&mut writer.lock(), &config, &files, &diagnostic)?;
            }
        }
    }

    Ok(())
}

fn emit_diagnostics(
    what: &str,
    path: &Path,
    source: &str,
    span: st::unit::Span,
    error: &(dyn Error + 'static),
) -> Result<()> {
    use codespan_reporting::diagnostic::{Diagnostic, Label};
    use codespan_reporting::files::SimpleFiles;
    use codespan_reporting::term;
    use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};

    let mut files = SimpleFiles::new();

    let source_file = files.add(path.display().to_string(), source);

    let mut current = Some(error);
    let mut labels = Vec::new();

    while let Some(e) = current {
        labels.push(Label::primary(source_file, span.start..span.end).with_message(e.to_string()));

        if let Some(cast) = e.downcast_ref::<rune::CompileError>() {
            match cast {
                rune::CompileError::ReturnLocalReferences {
                    block,
                    references_at,
                    span,
                    ..
                } => {
                    for ref_span in references_at {
                        if span.overlaps(*ref_span) {
                            continue;
                        }

                        labels.push(
                            Label::secondary(source_file, ref_span.start..ref_span.end)
                                .with_message("reference created here"),
                        );
                    }

                    labels.push(
                        Label::secondary(source_file, block.start..block.end)
                            .with_message("block returned from"),
                    );
                }
                _ => {}
            }
        }

        current = e.source();
    }

    let diagnostic = Diagnostic::error().with_message(what).with_labels(labels);

    let writer = StandardStream::stderr(ColorChoice::Always);
    let config = codespan_reporting::term::Config::default();

    term::emit(&mut writer.lock(), &config, &files, &diagnostic)?;
    Ok(())
}
