use std::fmt;
use std::io::Write;
use std::sync::Arc;
use std::time::Instant;

use clap::Parser;
use rune::compile::{Item, ItemBuf};
use rune::runtime::{Function, Unit, Value, VmResult};
use rune::{Any, Context, ContextError, Hash, Module, Sources};
use rune_modules::capture_io::CaptureIo;

use crate::{ExitCode, Io, SharedFlags};

#[derive(Parser, Debug, Clone)]
pub(crate) struct Flags {
    /// Rounds of warmup to perform
    #[arg(long, default_value = "100")]
    warmup: u32,

    /// Iterations to run of the benchmark
    #[arg(long, default_value = "100")]
    iterations: u32,

    #[command(flatten)]
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
    let mut module = Module::with_item(["std", "test"]);
    module.ty::<Bencher>()?;
    module.inst_fn("iter", Bencher::iter)?;
    Ok(module)
}

/// Run benchmarks.
pub(crate) async fn run(
    io: &mut Io<'_>,
    args: &Flags,
    context: &Context,
    capture_io: Option<&CaptureIo>,
    unit: Arc<Unit>,
    sources: &Sources,
    fns: &[(Hash, ItemBuf)],
) -> anyhow::Result<ExitCode> {
    let runtime = Arc::new(context.runtime());
    let mut vm = rune::Vm::new(runtime, unit);

    if fns.is_empty() {
        return Ok(ExitCode::Success);
    }

    writeln!(io.stdout, "Found {} benches...", fns.len())?;

    let mut any_error = false;

    for (hash, item) in fns {
        let mut bencher = Bencher::default();

        if let VmResult::Err(error) = vm.call(*hash, (&mut bencher,)) {
            writeln!(io.stdout, "{}: Error in benchmark", item)?;
            error.emit(io.stdout, sources)?;
            any_error = true;

            if let Some(capture_io) = capture_io {
                writeln!(io.stdout, "-- output --")?;
                capture_io.drain_into(&mut *io.stdout)?;
                writeln!(io.stdout, "-- end output --")?;
            }

            continue;
        }

        let multiple = bencher.fns.len() > 1;

        for (i, f) in bencher.fns.iter().enumerate() {
            if let Err(e) = bench_fn(io, i, item, args, f, multiple) {
                writeln!(io.stdout, "{}: Error in bench iteration: {}", item, e)?;

                if let Some(capture_io) = capture_io {
                    writeln!(io.stdout, "-- output --")?;
                    capture_io.drain_into(&mut *io.stdout)?;
                    writeln!(io.stdout, "-- end output --")?;
                }

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
    io: &mut Io<'_>,
    i: usize,
    item: &Item,
    args: &Flags,
    f: &Function,
    multiple: bool,
) -> anyhow::Result<()> {
    for _ in 0..args.warmup {
        let value = f.call::<_, Value>(()).into_result()?;
        drop(value);
    }

    let iterations = usize::try_from(args.iterations).expect("iterations out of bounds");
    let mut collected = Vec::with_capacity(iterations);

    for _ in 0..args.iterations {
        let start = Instant::now();
        let value = f.call::<_, Value>(()).into_result()?;
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

    let format = Format {
        average: average as u128,
        stddev: stddev as u128,
        iterations,
    };

    if multiple {
        writeln!(io.stdout, "bench {}#{}: {}", item, i, format)?;
    } else {
        writeln!(io.stdout, "bench {}: {}", item, format)?;
    }

    Ok(())
}

struct Format {
    average: u128,
    stddev: u128,
    iterations: usize,
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "mean={:.2}, stddev={:.2}, iterations={}",
            Time(self.average),
            Time(self.stddev),
            self.iterations
        )
    }
}

struct Time(u128);

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}ns", self.0)
    }
}
