use std::io::Write;
use std::sync::Arc;
use std::time::Instant;

use crate::no_std::prelude::*;

use anyhow::{Result, Context};
use clap::Parser;

use crate::cli::{ExitCode, Io, CommandBase, AssetKind, Config, SharedFlags, EntryPoint, Entry, Options};
use crate::cli::visitor;
use crate::cli::naming::Naming;
use crate::compile::{ItemBuf, FileSourceLoader};
use crate::modules::capture_io::CaptureIo;
use crate::runtime::{Value, Vm, VmError, VmResult};
use crate::{Hash, Sources, Unit, Diagnostics, Source};

#[derive(Parser, Debug, Clone)]
pub(super) struct Flags {
    /// Exit with a non-zero exit-code even for warnings
    #[arg(long)]
    warnings_are_errors: bool,
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

    let mut failures = 0usize;
    let mut executed = 0usize;

    let capture = crate::modules::capture_io::CaptureIo::new();
    let context = shared.context(entry, c, Some(&capture))?;

    let mut doc_visitors = Vec::new();
    let mut cases = Vec::new();
    let mut naming = Naming::default();

    let mut build_error = false;

    for e in entries {
        let name = naming.name(&e);
        let item = ItemBuf::with_crate(&name);

        let mut sources = Sources::new();

        let source = Source::from_path(e.path())
            .with_context(|| e.path().display().to_string())?;

        sources.insert(source);

        let mut diagnostics = if shared.warnings || flags.warnings_are_errors {
            Diagnostics::new()
        } else {
            Diagnostics::without_warnings()
        };

        let mut doc_visitor = crate::doc::Visitor::new(item);
        let mut functions = visitor::FunctionVisitor::new(visitor::Attribute::Test);
        let mut source_loader = FileSourceLoader::new();

        let unit = crate::prepare(&mut sources)
            .with_context(&context)
            .with_diagnostics(&mut diagnostics)
            .with_options(options)
            .with_visitor(&mut doc_visitor)
            .with_visitor(&mut functions)
            .with_source_loader(&mut source_loader)
            .build();

        diagnostics.emit(&mut io.stdout.lock(), &sources)?;

        if diagnostics.has_error() || flags.warnings_are_errors && diagnostics.has_warning() {
            build_error = true;
            continue;
        }

        let unit = Arc::new(unit?);
        let sources = Arc::new(sources);

        doc_visitors.push(doc_visitor);

        for (hash, item) in functions.into_functions() {
            cases.push(TestCase::new(hash, item, unit.clone(), sources.clone()));
        }
    }

    let mut artifacts = crate::doc::Artifacts::without_assets();
    crate::doc::build("root", &mut artifacts, &context, &doc_visitors)?;

    for test in artifacts.tests() {
        let mut sources = Sources::new();
        // TODO: allow compiling plain function content directly.
        let content = format!("pub async fn test_case() {{\n{}\n}}", test.content);
        let source = Source::new(test.item.to_string(), &content);
        sources.insert(source);

        let mut diagnostics = if shared.warnings || flags.warnings_are_errors {
            Diagnostics::new()
        } else {
            Diagnostics::without_warnings()
        };

        let mut source_loader = FileSourceLoader::new();

        let unit = crate::prepare(&mut sources)
            .with_context(&context)
            .with_diagnostics(&mut diagnostics)
            .with_options(options)
            .with_source_loader(&mut source_loader)
            .build();

        diagnostics.emit(&mut io.stdout.lock(), &sources)?;

        if diagnostics.has_error() || flags.warnings_are_errors && diagnostics.has_warning() {
            build_error = true;
            continue;
        }

        if !test.params.no_run {
            let unit = Arc::new(unit?);
            let sources = Arc::new(sources);
            cases.push(TestCase::new(Hash::type_hash(["test_case"]), test.item.clone(), unit.clone(), sources.clone()));
        }
    }

    if build_error {
        return Ok(ExitCode::Failure);
    }

    let runtime = Arc::new(context.runtime());

    for case in &mut cases {
        let mut vm = Vm::new(runtime.clone(), case.unit.clone());
    
        executed = executed.wrapping_add(1);

        let success = case.execute(io, &mut vm, flags.quiet, Some(&capture)).await?;

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
        case.emit(io)?;
    }

    let elapsed = start.elapsed();

    writeln!(
        io.stdout,
        "Executed {} tests with {} failures ({} skipped) in {:.3} seconds",
        executed,
        failures,
        cases.len() - executed,
        elapsed.as_secs_f64()
    )?;

    if failures == 0 {
        Ok(ExitCode::Success)
    } else {
        Ok(ExitCode::Failure)
    }
}

#[derive(Debug)]
enum FailureReason {
    Crash(VmError),
    ReturnedNone,
    ReturnedErr { output: Box<[u8]>, error: Value },
}

struct TestCase {
    hash: Hash,
    item: ItemBuf,
    unit: Arc<Unit>,
    sources: Arc<Sources>,
    outcome: Option<FailureReason>,
    buf: Vec<u8>,
}

impl TestCase {
    fn new(hash: Hash, item: ItemBuf, unit: Arc<Unit>, sources: Arc<Sources>) -> Self {
        Self {
            hash,
            item,
            unit,
            sources,
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
                Some(FailureReason::Crash(..)) => {
                    writeln!(io.stdout, "F")?;
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
                Some(FailureReason::Crash(..)) => {
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

    fn emit(&self, io: &mut Io<'_>) -> Result<()> {
        if let Some(outcome) = &self.outcome {
            match outcome {
                FailureReason::Crash(err) => {
                    writeln!(io.stdout, "----------------------------------------")?;
                    writeln!(io.stdout, "Test: {}\n", self.item)?;
                    err.emit(io.stdout, &self.sources)?;
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
