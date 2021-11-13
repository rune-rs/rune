use crate::ExitCode;
use rune::compile::Item;
use rune::meta::CompileMeta;
use rune::runtime::{Function, RuntimeContext, Unit, Value};
use rune::termcolor::StandardStream;
use rune::{Any, ContextError, Hash, Module, Sources};
use std::io::Write;
use std::sync::Arc;
use std::time::Instant;

#[derive(Default, Any)]
pub(crate) struct Bencher {
    fns: Vec<Function>,
}

impl Bencher {
    fn iter(&mut self, f: Function) {
        self.fns.push(f);
    }
}

/// Registers `std::test` module.
pub(crate) fn test_module() -> Result<Module, ContextError> {
    let mut module = Module::with_item(&["std", "test"]);
    module.ty::<Bencher>()?;
    module.inst_fn("iter", Bencher::iter)?;
    Ok(module)
}

/// Run benchmarks.
pub(crate) async fn do_benches(
    args: &crate::BenchFlags,
    mut out: StandardStream,
    runtime: Arc<RuntimeContext>,
    unit: Arc<Unit>,
    sources: Sources,
    found: Vec<(Hash, CompileMeta)>,
) -> anyhow::Result<ExitCode> {
    let mut vm = rune::Vm::new(runtime, unit.clone());

    writeln!(out, "Found {} benches...", found.len())?;

    let mut any_error = false;

    for (hash, meta) in found {
        let mut bencher = Bencher::default();

        if let Err(error) = vm.call(hash, (&mut bencher,)) {
            writeln!(out, "Error in benchmark `{}`", meta.item.item)?;
            error.emit(&mut out, &sources)?;
            any_error = true;
            continue;
        }

        for (i, f) in bencher.fns.iter().enumerate() {
            if let Err(e) = bench_fn(&mut out, i, &meta.item.item, args, f) {
                writeln!(out, "Error running benchmark iteration: {}", e)?;
                any_error = true;
            }
        }
    }

    if any_error {
        Ok(ExitCode::Failure)
    } else {
        Ok(ExitCode::Success)
    }
}

fn bench_fn(
    out: &mut StandardStream,
    i: usize,
    item: &Item,
    args: &crate::BenchFlags,
    f: &Function,
) -> anyhow::Result<()> {
    for _ in 0..args.warmup {
        let value = f.call::<_, Value>(())?;
        drop(value);
    }

    let mut collected = Vec::with_capacity(args.iterations as usize);

    for _ in 0..args.iterations {
        let start = Instant::now();
        let value = f.call::<_, Value>(())?;
        let duration = Instant::now().duration_since(start);
        collected.push(duration.as_nanos() as i128);
        drop(value);
    }

    collected.sort();

    let len = collected.len() as f64;
    let average = collected.iter().copied().sum::<i128>() as f64 / len;
    let variance = collected
        .iter()
        .copied()
        .map(|n| (n as f64 - average).powf(2.0))
        .sum::<f64>()
        / len;
    let stddev = variance.sqrt();

    writeln!(
        out,
        "bench {}#{}: mean={:.2}ns, stddev={:.2}, iterations={}",
        item, i, average, stddev, args.iterations,
    )?;
    Ok(())
}
