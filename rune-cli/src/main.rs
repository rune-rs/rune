use anyhow::{bail, Result};
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use st_frontend::Encode as _;
use st_frontend_rune::{ast, parse_all, SpannedError as _};

fn compile(source: &str) -> st_frontend_rune::Result<st::Unit> {
    let unit = parse_all::<ast::File>(&source)?;
    Ok(unit.encode()?)
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let mut args = env::args();
    args.next();

    let mut path = None;
    let mut debug = false;

    for arg in args {
        match arg.as_str() {
            "--debug" => {
                debug = true;
            }
            other => {
                path = Some(PathBuf::from(other));
            }
        }
    }

    let path = match path {
        Some(path) => PathBuf::from(path),
        None => {
            bail!("expected: rune-cli [--debug] <file>");
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

    if debug {
        println!("unit: {:?}", unit);
    }

    let mut vm = st::Vm::new();
    let functions = st::Functions::new();

    let mut task: st::Task<u128> = vm.call_function(&functions, &unit, "main", ())?;

    let last = std::time::Instant::now();

    let result = loop {
        if debug {
            println!("ip = {}, state = {:?}", task.ip, task.vm);
            println!("next = {:?}", task.unit.instructions.get(task.ip));
        }

        let result = task.step().await;

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
    println!("result = {:?} ({:?})", result, duration);
    Ok(())
}
