use std::fmt;
use std::hint;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use crate::alloc::Vec;
use crate::cli::{AssetKind, CommandBase, Config, ExitCode, Io, SharedFlags};
use crate::modules::capture_io::CaptureIo;
use crate::modules::test::Bencher;
use crate::runtime::{Function, Unit, Value};
use crate::support::Result;
use crate::{Context, Hash, Item, ItemBuf, Sources, Vm};

mod cli {
    use std::path::PathBuf;
    use std::vec::Vec;

    use clap::Parser;

    #[derive(Parser, Debug)]
    #[command(rename_all = "kebab-case")]
    pub(crate) struct Flags {
        /// Rounds of warmup to perform
        #[arg(long, default_value = "100")]
        pub(super) warmup: u32,
        /// Iterations to run of the benchmark
        #[arg(long, default_value = "100")]
        pub(super) iter: u32,
        /// Explicit paths to benchmark.
        pub(super) bench_path: Vec<PathBuf>,
    }
}

pub(super) use cli::Flags;

impl CommandBase for Flags {
    #[inline]
    fn is_workspace(&self, kind: AssetKind) -> bool {
        matches!(kind, AssetKind::Bench)
    }

    #[inline]
    fn describe(&self) -> &str {
        "Benchmarking"
    }

    #[inline]
    fn propagate(&mut self, c: &mut Config, _: &mut SharedFlags) {
        c.test = true;
    }

    #[inline]
    fn paths(&self) -> &[PathBuf] {
        &self.bench_path
    }
}

/// Run benchmarks.
pub(super) async fn run(
    io: &mut Io<'_>,
    args: &Flags,
    context: &Context,
    capture_io: Option<&CaptureIo>,
    unit: Arc<Unit>,
    sources: &Sources,
    fns: &[(Hash, ItemBuf)],
) -> Result<ExitCode> {
    let runtime = Arc::new(context.runtime()?);
    let mut vm = Vm::new(runtime, unit);

    if fns.is_empty() {
        return Ok(ExitCode::Success);
    }

    writeln!(io.stdout, "Found {} benches...", fns.len())?;

    let mut any_error = false;

    for (hash, item) in fns {
        let mut bencher = Bencher::default();

        if let Err(error) = vm.call(*hash, (&mut bencher,)) {
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

        let fns = bencher.into_functions();

        let multiple = fns.len() > 1;

        for (i, f) in fns.iter().enumerate() {
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
) -> Result<()> {
    for _ in 0..args.warmup {
        let value = f.call::<Value>(()).into_result()?;
        drop(hint::black_box(value));
    }

    let iterations = usize::try_from(args.iter).expect("iterations out of bounds");
    let mut collected = Vec::try_with_capacity(iterations)?;

    for _ in 0..args.iter {
        let start = Instant::now();
        let value = f.call::<Value>(()).into_result()?;
        let duration = Instant::now().duration_since(start);
        collected.try_push(duration.as_nanos() as i128)?;
        drop(hint::black_box(value));
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
        if self.0 >= 1_000_000_000 {
            write!(f, "{:.3}s", self.0 as f64 / 1_000_000_000.0)
        } else if self.0 >= 1_000_000 {
            write!(f, "{:.3}ms", self.0 as f64 / 1_000_000.0)
        } else if self.0 >= 1_000 {
            write!(f, "{:.3}µs", self.0 as f64 / 1_000.0)
        } else {
            write!(f, "{}ns", self.0)
        }
    }
}
