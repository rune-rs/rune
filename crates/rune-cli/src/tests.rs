use std::io::Write;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use clap::Parser;
use rune::compile::ItemBuf;
use rune::runtime::{Unit, Value, Vm, VmError};
use rune::{Context, Hash, Sources};
use rune_modules::capture_io::CaptureIo;

use crate::{ExitCode, Io, SharedFlags};

#[derive(Parser, Debug, Clone)]
pub(crate) struct Flags {
    /// Display one character per test instead of one line
    #[arg(long, short = 'q')]
    quiet: bool,

    /// Run all tests regardless of failure
    #[arg(long)]
    no_fail_fast: bool,

    #[command(flatten)]
    pub(crate) shared: SharedFlags,
}

#[derive(Debug)]
enum FailureReason {
    Crash(VmError),
    ReturnedNone,
    ReturnedErr { output: Box<[u8]>, error: Value },
}

#[derive(Debug)]
struct TestCase<'a> {
    hash: Hash,
    item: &'a ItemBuf,
    outcome: Option<FailureReason>,
    buf: Vec<u8>,
}

impl<'a> TestCase<'a> {
    fn from_parts(hash: Hash, item: &'a ItemBuf) -> Self {
        Self {
            hash,
            item,
            outcome: None,
            buf: Vec::new(),
        }
    }

    async fn execute(
        &mut self,
        io: &mut Io<'_>,
        vm: &mut Vm,
        quiet: bool,
        capture_io: Option<&CaptureIo>,
    ) -> Result<bool> {
        if !quiet {
            write!(io.stdout, "Test {:30} ", self.item)?;
        }

        let result = match vm.execute(self.hash, ()) {
            Err(err) => Err(err),
            Ok(mut execution) => execution.async_complete().await,
        };

        if let Some(capture_io) = capture_io {
            let _ = capture_io.drain_into(&mut self.buf);
        }

        self.outcome = match result {
            Err(e) => Some(FailureReason::Crash(e)),
            Ok(v) => match v {
                Value::Result(result) => match result.take()? {
                    Ok(..) => None,
                    Err(error) => Some(FailureReason::ReturnedErr {
                        output: self.buf.as_slice().into(),
                        error,
                    }),
                },
                Value::Option(option) => match *option.borrow_ref()? {
                    Some(..) => None,
                    None => Some(FailureReason::ReturnedNone),
                },
                _ => None,
            },
        };

        if quiet {
            match &self.outcome {
                Some(FailureReason::Crash { .. }) => {
                    write!(io.stdout, "F")?;
                }
                Some(FailureReason::ReturnedErr { .. }) => {
                    write!(io.stdout, "f")?;
                }
                Some(FailureReason::ReturnedNone { .. }) => {
                    write!(io.stdout, "n")?;
                }
                None => {
                    write!(io.stdout, ".")?;
                }
            }
        } else {
            match &self.outcome {
                Some(FailureReason::Crash { .. }) => {
                    writeln!(io.stdout, "failed")?;
                }
                Some(FailureReason::ReturnedErr { .. }) => {
                    writeln!(io.stdout, "returned error")?;
                }
                Some(FailureReason::ReturnedNone { .. }) => {
                    writeln!(io.stdout, "returned none")?;
                }
                None => {
                    writeln!(io.stdout, "passed")?;
                }
            }
        }

        self.buf.clear();
        Ok(self.outcome.is_none())
    }

    fn emit(&self, io: &mut Io<'_>, sources: &Sources) -> Result<()> {
        if let Some(outcome) = &self.outcome {
            match outcome {
                FailureReason::Crash(err) => {
                    writeln!(io.stdout, "----------------------------------------")?;
                    writeln!(io.stdout, "Test: {}\n", self.item)?;
                    err.emit(io.stdout, sources)?;
                }
                FailureReason::ReturnedNone { .. } => {}
                FailureReason::ReturnedErr { output, error, .. } => {
                    writeln!(io.stdout, "----------------------------------------")?;
                    writeln!(io.stdout, "Test: {}\n", self.item)?;
                    writeln!(io.stdout, "Error: {:?}\n", error)?;
                    writeln!(io.stdout, "-- output --")?;
                    io.stdout.write_all(output)?;
                    writeln!(io.stdout, "-- end of output --")?;
                }
            }
        }

        Ok(())
    }
}

pub(crate) async fn run(
    io: &mut Io<'_>,
    flags: &Flags,
    context: &Context,
    capture_io: Option<&CaptureIo>,
    unit: Arc<Unit>,
    sources: &Sources,
    fns: &[(Hash, ItemBuf)],
) -> anyhow::Result<ExitCode> {
    let runtime = Arc::new(context.runtime());

    let mut cases = fns
        .iter()
        .map(|v| TestCase::from_parts(v.0, &v.1))
        .collect::<Vec<_>>();

    if cases.is_empty() {
        return Ok(ExitCode::Success);
    }

    writeln!(io.stdout, "Found {} tests...", cases.len())?;

    let start = Instant::now();
    let mut failure_count = 0;
    let mut executed_count = 0;

    let mut vm = Vm::new(runtime.clone(), unit.clone());

    for test in &mut cases {
        executed_count += 1;

        let success = test.execute(io, &mut vm, flags.quiet, capture_io).await?;

        if !success {
            failure_count += 1;

            if !flags.no_fail_fast {
                break;
            }
        }
    }

    if flags.quiet {
        writeln!(io.stdout)?;
    }

    let elapsed = start.elapsed();

    for case in &cases {
        case.emit(io, sources)?;
    }

    writeln!(io.stdout, "====")?;
    writeln!(
        io.stdout,
        "Executed {} tests with {} failures ({} skipped) in {:.3} seconds",
        executed_count,
        failure_count,
        cases.len() - executed_count,
        elapsed.as_secs_f64()
    )?;

    if failure_count == 0 {
        Ok(ExitCode::Success)
    } else {
        Ok(ExitCode::Failure)
    }
}
