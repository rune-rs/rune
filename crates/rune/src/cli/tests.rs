use std::io::Write;
use std::sync::Arc;
use std::time::Instant;

use crate::no_std::prelude::*;

use anyhow::Result;
use clap::Parser;

use crate::cli::{ExitCode, Io, CommandBase, AssetKind, Config, SharedFlags, EntryPoint, Entry, Options};
use crate::cli::loader;
use crate::cli::visitor;
use crate::compile::ItemBuf;
use crate::modules::capture_io::CaptureIo;
use crate::runtime::{Value, Vm, VmError, VmResult};
use crate::{Hash, Sources};

#[derive(Parser, Debug, Clone)]
pub(super) struct Flags {
    /// Display one character per test instead of one line
    #[arg(long, short = 'q')]
    quiet: bool,

    /// Run all tests regardless of failure
    #[arg(long)]
    no_fail_fast: bool,
}

impl CommandBase for Flags {
    #[inline]
    fn is_debug(&self) -> bool {
        true
    }

    #[inline]
    fn is_workspace(&self, kind: AssetKind) -> bool {
        matches!(kind, AssetKind::Test)
    }

    #[inline]
    fn describe(&self) -> &str {
        "Testing"
    }

    #[inline]
    fn propagate(&mut self, c: &mut Config, _: &mut SharedFlags) {
        c.test = true;
    }
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
            write!(io.stdout, "{} ", self.item)?;
        }

        let result = match vm.execute(self.hash, ()) {
            Ok(mut execution) => execution.async_complete().await,
            Err(err) => VmResult::Err(err),
        };

        if let Some(capture_io) = capture_io {
            let _ = capture_io.drain_into(&mut self.buf);
        }

        self.outcome = match result {
            VmResult::Ok(v) => match v {
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
            VmResult::Err(e) => Some(FailureReason::Crash(e)),
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

/// Run all tests that can be found.
pub(super) async fn run<'p, I>(
    io: &mut Io<'_>,
    c: &Config,
    flags: &Flags,
    shared: &SharedFlags,
    options: &Options,
    entry: &mut Entry<'_>,
    entries: I,
) -> anyhow::Result<ExitCode>
where
    I: IntoIterator<Item = EntryPoint<'p>>,
{
    let start = Instant::now();

    let mut total = 0usize;
    let mut failures = 0usize;
    let mut executed = 0usize;

    let capture = crate::modules::capture_io::CaptureIo::new();
    let context = shared.context(entry, c, Some(&capture))?;

    for e in entries {
        let load = loader::load(
            io,
            &context,
            shared,
            options,
            e.path(),
            visitor::Attribute::Test,
        )?;

        let runtime = Arc::new(context.runtime());

        let mut cases = load.functions
            .iter()
            .map(|v| TestCase::from_parts(v.0, &v.1))
            .collect::<Vec<_>>();

        if cases.is_empty() {
            continue;
        }

        total = total.wrapping_add(cases.len());
    
        let mut vm = Vm::new(runtime.clone(), load.unit.clone());
    
        for test in &mut cases {
            executed = executed.wrapping_add(1);
    
            let success = test.execute(io, &mut vm, flags.quiet, Some(&capture)).await?;
    
            if !success {
                failures = failures.wrapping_add(1);
    
                if !flags.no_fail_fast {
                    break;
                }
            }
        }
    
        if flags.quiet {
            writeln!(io.stdout)?;
        }
    
        for case in &cases {
            case.emit(io, &load.sources)?;
        }
    }

    let elapsed = start.elapsed();
    
    writeln!(io.stdout, "====")?;

    writeln!(
        io.stdout,
        "Executed {} tests with {} failures ({} skipped) in {:.3} seconds",
        executed,
        failures,
        total - executed,
        elapsed.as_secs_f64()
    )?;

    if failures == 0 {
        Ok(ExitCode::Success)
    } else {
        Ok(ExitCode::Failure)
    }
}
