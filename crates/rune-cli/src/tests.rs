use crate::{ExitCode, SharedFlags};
use rune::compile::Meta;
use rune::runtime::{Unit, Value, Vm, VmError};
use rune::termcolor::StandardStream;
use rune::{Context, Hash, Sources};
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
    ReturnedErr(Value),
}

#[derive(Debug)]
struct TestCase<'a> {
    hash: Hash,
    meta: &'a Meta,
    outcome: Option<FailureReason>,
}

impl<'a> TestCase<'a> {
    fn from_parts(hash: Hash, meta: &'a Meta) -> Self {
        Self {
            hash,
            meta,
            outcome: None,
        }
    }

    fn start(&self, o: &mut StandardStream, quiet: bool) -> Result<(), std::io::Error> {
        if quiet {
            return Ok(());
        }

        write!(o, "Test {:30} ", self.meta.item.item)
    }

    async fn execute(&mut self, vm: &mut Vm) -> Result<bool, VmError> {
        let result = match vm.execute(self.hash, ()) {
            Err(err) => Err(err),
            Ok(mut execution) => execution.async_complete().await,
        };

        self.outcome = match result {
            Err(e) => Some(FailureReason::Crash(e)),
            Ok(v) => match v {
                Value::Result(result) => match result.take()? {
                    Ok(..) => None,
                    Err(err) => Some(FailureReason::ReturnedErr(err)),
                },
                Value::Option(option) => match *option.borrow_ref()? {
                    Some(..) => None,
                    None => Some(FailureReason::ReturnedNone),
                },
                _ => None,
            },
        };

        Ok(self.outcome.is_none())
    }

    fn end(&self, o: &mut StandardStream, quiet: bool) -> Result<(), std::io::Error> {
        if quiet {
            match &self.outcome {
                Some(FailureReason::Crash(_)) => {
                    write!(o, "F")
                }
                Some(FailureReason::ReturnedErr(_)) => {
                    write!(o, "f")
                }
                Some(FailureReason::ReturnedNone) => {
                    write!(o, "n")
                }
                None => write!(o, "."),
            }
        } else {
            match &self.outcome {
                Some(FailureReason::Crash(_)) => {
                    writeln!(o, "failed")
                }
                Some(FailureReason::ReturnedErr(_)) => {
                    writeln!(o, "returned error")
                }
                Some(FailureReason::ReturnedNone) => {
                    writeln!(o, "returned none")
                }
                None => writeln!(o, "passed"),
            }
        }
    }

    fn emit(&self, o: &mut StandardStream, sources: &Sources) -> Result<(), std::io::Error> {
        if self.outcome.is_none() {
            return Ok(());
        }
        match self.outcome.as_ref().unwrap() {
            FailureReason::Crash(err) => {
                writeln!(o, "----------------------------------------")?;
                writeln!(o, "Test: {}\n", self.meta.item.item)?;
                err.emit(o, sources).expect("failed writing diagnostics");
            }
            FailureReason::ReturnedNone => {}
            FailureReason::ReturnedErr(e) => {
                writeln!(o, "----------------------------------------")?;
                writeln!(o, "Test: {}\n", self.meta.item.item)?;
                writeln!(o, "Error: {:?}\n", e)?;
            }
        }
        Ok(())
    }
}

pub(crate) async fn run(
    o: &mut StandardStream,
    flags: &Flags,
    context: &Context,
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

        test.start(o, flags.quiet)?;
        let success = test.execute(&mut vm).await?;
        test.end(o, flags.quiet)?;
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
