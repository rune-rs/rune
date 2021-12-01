use crate::{ExitCode, SharedFlags};
use rune::compile::{Item, Meta};
use rune::runtime::{Function, Unit, Value};
use rune::termcolor::StandardStream;
use rune::{Any, Context, ContextError, Hash, Module, Sources};
use std::io::Write;
use std::sync::Arc;
use std::time::Instant;
use structopt::StructOpt;

#[derive(StructOpt, Debug, Clone)]
pub(crate) struct Flags {
    /// Rounds of warmup to perform
    #[structopt(long, default_value = "100")]
    warmup: u32,

    /// Iterations to run of the benchmark
    #[structopt(long, default_value = "100")]
    iterations: u32,

    #[structopt(flatten)]
    pub(crate) shared: SharedFlags,
}

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
pub(crate) async fn run(
    o: &mut StandardStream,
    args: &Flags,
    context: &Context,
    unit: Arc<Unit>,
    sources: &Sources,
    fns: &[(Hash, Meta)],
) -> anyhow::Result<ExitCode> {
    let runtime = Arc::new(context.runtime());
    let mut vm = rune::Vm::new(runtime, unit);

    writeln!(o, "Found {} benches...", fns.len())?;

    let mut any_error = false;

    for (hash, meta) in fns {
        let mut bencher = Bencher::default();

        if let Err(error) = vm.call(*hash, (&mut bencher,)) {
            writeln!(o, "Error in benchmark `{}`", meta.item.item)?;
            error.emit(o, sources)?;
            any_error = true;
            continue;
        }

        for (i, f) in bencher.fns.iter().enumerate() {
            if let Err(e) = bench_fn(o, i, &meta.item.item, args, f) {
                writeln!(o, "Error running benchmark iteration: {}", e)?;
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
    o: &mut StandardStream,
    i: usize,
    item: &Item,
    args: &Flags,
    f: &Function,
) -> anyhow::Result<()> {
    for _ in 0..args.warmup {
        let value = f.call::<_, Value>(())?;
        drop(value);
    }

    let iterations = usize::try_from(args.iterations).expect("iterations out of bounds");
    let mut collected = Vec::with_capacity(iterations);

    for _ in 0..args.iterations {
        let start = Instant::now();
        let value = f.call::<_, Value>(())?;
        let duration = Instant::now().duration_since(start);
        collected.push(duration.as_nanos() as i128);
        drop(value);
    }

    collected.sort_unstable();

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
        o,
        "bench {}#{}: mean={:.2}ns, stddev={:.2}, iterations={}",
        item, i, average, stddev, args.iterations,
    )?;
    Ok(())
}
