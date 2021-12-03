use crate::{ExitCode, SharedFlags};
use anyhow::Result;
use rune::compile::Meta;
use rune::runtime::{Unit, Value, Vm, VmError};
use rune::termcolor::StandardStream;
use rune::{Context, Hash, Sources};
use rune_modules::capture_io::CaptureIo;
use std::io::Write;
use std::sync::Arc;
use std::time::Instant;
use structopt::StructOpt;

#[derive(StructOpt, Debug, Clone)]
pub(crate) struct Flags {
    /// Display one character per test instead of one line
    #[structopt(short = "q", long)]
    quiet: bool,

    /// Run all tests regardless of failure
    #[structopt(long)]
    no_fail_fast: bool,

    #[structopt(flatten)]
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
    meta: &'a Meta,
    outcome: Option<FailureReason>,
    buf: Vec<u8>,
}

impl<'a> TestCase<'a> {
    fn from_parts(hash: Hash, meta: &'a Meta) -> Self {
        Self {
            hash,
            meta,
            outcome: None,
            buf: Vec::new(),
        }
    }

    async fn execute(
        &mut self,
        o: &mut StandardStream,
        vm: &mut Vm,
        quiet: bool,
        io: Option<&CaptureIo>,
    ) -> Result<bool> {
        if !quiet {
            write!(o, "Test {:30} ", self.meta.item.item)?;
        }

        let result = match vm.execute(self.hash, ()) {
            Err(err) => Err(err),
            Ok(mut execution) => execution.async_complete().await,
        };

        if let Some(io) = io {
            let _ = io.drain_into(&mut self.buf);
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
                    write!(o, "F")?;
                }
                Some(FailureReason::ReturnedErr { .. }) => {
                    write!(o, "f")?;
                }
                Some(FailureReason::ReturnedNone { .. }) => {
                    write!(o, "n")?;
                }
                None => {
                    write!(o, ".")?;
                }
            }
        } else {
            match &self.outcome {
                Some(FailureReason::Crash { .. }) => {
                    writeln!(o, "failed")?;
                }
                Some(FailureReason::ReturnedErr { .. }) => {
                    writeln!(o, "returned error")?;
                }
                Some(FailureReason::ReturnedNone { .. }) => {
                    writeln!(o, "returned none")?;
                }
                None => {
                    writeln!(o, "passed")?;
                }
            }
        }

        self.buf.clear();
        Ok(self.outcome.is_none())
    }

    fn emit(&self, o: &mut StandardStream, sources: &Sources) -> Result<()> {
        if let Some(outcome) = &self.outcome {
            match outcome {
                FailureReason::Crash(err) => {
                    writeln!(o, "----------------------------------------")?;
                    writeln!(o, "Test: {}\n", self.meta.item.item)?;
                    err.emit(o, sources)?;
                }
                FailureReason::ReturnedNone { .. } => {}
                FailureReason::ReturnedErr { output, error, .. } => {
                    writeln!(o, "----------------------------------------")?;
                    writeln!(o, "Test: {}\n", self.meta.item.item)?;
                    writeln!(o, "Error: {:?}\n", error)?;
                    writeln!(o, "-- output --")?;
                    o.write_all(output)?;
                    writeln!(o, "-- end of output --")?;
                }
            }
        }

        Ok(())
    }
}

pub(crate) async fn run(
    o: &mut StandardStream,
    flags: &Flags,
    context: &Context,
    io: Option<&CaptureIo>,
    unit: Arc<Unit>,
    sources: &Sources,
    fns: &[(Hash, Meta)],
) -> anyhow::Result<ExitCode> {
    let runtime = Arc::new(context.runtime());

    let mut cases = fns
        .iter()
        .map(|v| TestCase::from_parts(v.0, &v.1))
        .collect::<Vec<_>>();

    writeln!(o, "Found {} tests...", cases.len())?;

    let start = Instant::now();
    let mut failure_count = 0;
    let mut executed_count = 0;

    let mut vm = Vm::new(runtime.clone(), unit.clone());

    for test in &mut cases {
        executed_count += 1;

        let success = test.execute(o, &mut vm, flags.quiet, io).await?;

        if !success {
            failure_count += 1;

            if !flags.no_fail_fast {
                break;
            }
        }
    }

    if flags.quiet {
        writeln!(o)?;
    }

    let elapsed = start.elapsed();

    for case in &cases {
        case.emit(o, sources)?;
    }

    writeln!(o, "====")?;
    writeln!(
        o,
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
