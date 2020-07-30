use anyhow::{bail, Result};
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use rune::SpannedError as _;

fn compile(source: &str) -> rune::Result<st::Unit> {
    let unit = rune::parse_all::<rune::ast::File>(&source)?;
    Ok(unit.encode()?)
}

fn main() -> Result<()> {
    env_logger::init();

    let mut runtime = tokio::runtime::Runtime::new()?;

    let mut args = env::args();
    args.next();
    args.next();

    let mut path = None;
    let mut trace = false;
    let mut dump_unit = false;
    let mut dump_vm = false;
    let mut dump_functions = false;
    let mut help = false;

    for arg in args {
        match arg.as_str() {
            "--trace" => {
                trace = true;
            }
            "--dump" => {
                dump_unit = true;
                dump_vm = true;
                dump_functions = true;
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
            "--help" => {
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
        println!("  --trace          - Provide detailed tracing for each instruction executed.");
        println!("  --dump           - Dump all forms of diagnostic.");
        println!("  --dump-unit      - Dump diagnostics on the unit generated from the file.");
        println!("  --dump-vm-state  - Dump diagnostics on VM state (stack).");
        println!("  --dump-functions - Dump available functions.");
        return Ok(());
    }

    let path = match path {
        Some(path) => PathBuf::from(path),
        None => {
            bail!("Invalid usage: {}", USAGE);
        }
    };

    let source = fs::read_to_string(&path)?;

    let unit = match compile(&source) {
        Ok(unit) => unit,
        Err(e) => {
            let span = e.span();
            let thing = &source[span.start..span.end];
            let (line, col) = span.line_col(&source);

            println!(
                "{} at {}:{}:{} {}",
                e,
                path.display(),
                line + 1,
                col + 1,
                thing
            );

            let mut i = 0;
            let mut e = &e as &dyn Error;

            while let Some(err) = e.source() {
                println!("#{}: {}", i, err);
                i += 1;
                e = err;
            }

            return Ok(());
        }
    };

    let mut functions = st::Functions::with_default_packages()?;
    functions.install(st_http::module()?)?;
    functions.install(st_json::module()?)?;

    if dump_functions {
        println!("# functions");

        for (i, (hash, f)) in functions.iter_functions().enumerate() {
            println!("{:04} = {} ({})", i, f, hash);
        }
    }

    if dump_unit {
        println!("# unit dump");
        println!("instructions:");

        for (i, inst) in unit.iter_instructions().enumerate() {
            println!("{:04} = {:?}", i, inst);
        }

        println!("imports:");

        for (hash, f) in unit.iter_imports() {
            println!("{} = {}", hash, f);
        }

        println!("functions:");

        for (hash, f) in unit.iter_functions() {
            println!("{} = {:?}", hash, f);
        }

        println!("strings:");

        for (hash, string) in unit.iter_static_strings() {
            println!("{} = {:?}", hash, string);
        }

        println!("---");
    }

    let mut vm = st::Vm::new();

    let mut task: st::Task<st::Value> = vm.call_function(&functions, &unit, &["main"], ())?;

    let last = std::time::Instant::now();

    let result = loop {
        if trace {
            if let Some(inst) = task.unit.instruction_at(task.vm.ip()) {
                println!("{:04} = {:?}", task.vm.ip(), inst,);
            } else {
                println!("{:04} = *out of bounds*", task.vm.ip(),);
            }
        }

        let result = runtime.block_on(task.step());

        if trace && dump_vm {
            println!("# stack dump");

            for (n, (slot, value)) in task.vm.iter_stack_debug().enumerate() {
                println!("{} = {:?} ({:?})", n, slot, value);
            }

            println!("---");
        }

        let result = match result {
            Ok(result) => result,
            Err(e) => {
                println!("#0: {}", e);
                let mut e = &e as &dyn Error;
                let mut i = 1;

                while let Some(err) = e.source() {
                    println!("#{}: {}", i, err);
                    i += 1;
                    e = err;
                }

                return Ok(());
            }
        };

        if let Some(result) = result {
            break result;
        }
    };

    let duration = std::time::Instant::now().duration_since(last);
    println!("== {:?} ({:?})", result, duration);

    if dump_vm {
        println!("# stack dump after completion");

        for (n, (slot, value)) in vm.iter_stack_debug().enumerate() {
            println!("{} = {:?} ({:?})", n, slot, value);
        }

        println!("---");
    }

    Ok(())
}
