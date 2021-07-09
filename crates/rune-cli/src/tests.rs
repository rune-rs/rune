use crate::ExitCode;
use rune::{termcolor::StandardStream, EmitDiagnostics, Sources};
use runestick::{
    CompileMeta, CompileMetaKind, Hash, RuntimeContext, Unit, UnitFn, Value, Vm, VmError,
    VmErrorKind,
};
use std::{cell::RefCell, io::Write, sync::Arc, time::Instant};

#[derive(Default)]
pub struct TestVisitor {
    test_functions: RefCell<Vec<(Hash, CompileMeta)>>,
}

impl TestVisitor {
    /// Convert visitor into test functions.
    pub(crate) fn into_test_functions(self) -> Vec<(Hash, CompileMeta)> {
        self.test_functions.into_inner()
    }
}

impl rune::CompileVisitor for TestVisitor {
    fn register_meta(&self, meta: &CompileMeta) {
        let type_hash = match &meta.kind {
            CompileMetaKind::Function { is_test, type_hash } if *is_test => type_hash,
            _ => return,
        };

        self.test_functions
            .borrow_mut()
            .push((*type_hash, meta.clone()));
    }
}

#[derive(Debug)]
enum FailureReason {
    Crash(VmError),
    ReturnedNone,
    ReturnedErr(Result<Value, Value>),
}

#[derive(Debug)]
struct TestCase {
    hash: Hash,
    meta: CompileMeta,
    outcome: Option<FailureReason>,
}

impl TestCase {
    fn from_parts(hash: Hash, meta: CompileMeta) -> Self {
        Self {
            hash,
            meta,
            outcome: None,
        }
    }

    fn start(&self, out: &mut StandardStream, quiet: bool) -> Result<(), std::io::Error> {
        if quiet {
            return Ok(());
        }

        write!(out, "Test {:30} ", self.meta.item.item)
    }

    async fn execute(&mut self, unit: &Unit, mut vm: Vm) -> Result<bool, VmError> {
        let info = unit.lookup(self.hash).ok_or_else(|| {
            VmError::from(VmErrorKind::MissingEntry {
                hash: self.hash,
                item: self.meta.item.item.clone(),
            })
        })?;

        let offset = match info {
            // NB: we ignore the calling convention.
            // everything is just async when called externally.
            UnitFn::Offset { offset, .. } => offset,
            _ => {
                return Err(VmError::from(VmErrorKind::MissingFunction {
                    hash: self.hash,
                }));
            }
        };

        vm.set_ip(offset);
        self.outcome = match vm.async_complete().await {
            Err(e) => Some(FailureReason::Crash(e)),
            Ok(v) => {
                if let Ok(v) = v.clone().into_result() {
                    let res = v.take().unwrap();
                    if res.is_err() {
                        Some(FailureReason::ReturnedErr(res))
                    } else {
                        None
                    }
                } else if let Ok(v) = v.into_option() {
                    if v.borrow_ref().unwrap().is_none() {
                        Some(FailureReason::ReturnedNone)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        };

        Ok(self.outcome.is_none())
    }

    fn end(&self, out: &mut StandardStream, quiet: bool) -> Result<(), std::io::Error> {
        if quiet {
            match &self.outcome {
                Some(FailureReason::Crash(_)) => {
                    write!(out, "F")
                }
                Some(FailureReason::ReturnedErr(_)) => {
                    write!(out, "f")
                }
                Some(FailureReason::ReturnedNone) => {
                    write!(out, "n")
                }
                None => write!(out, "."),
            }
        } else {
            match &self.outcome {
                Some(FailureReason::Crash(_)) => {
                    writeln!(out, "failed")
                }
                Some(FailureReason::ReturnedErr(_)) => {
                    writeln!(out, "returned error")
                }
                Some(FailureReason::ReturnedNone) => {
                    writeln!(out, "returned none")
                }
                None => writeln!(out, "passed"),
            }
        }
    }

    fn emit_diagnostics(
        &self,
        out: &mut StandardStream,
        sources: &Sources,
    ) -> Result<(), std::io::Error> {
        if self.outcome.is_none() {
            return Ok(());
        }
        match self.outcome.as_ref().unwrap() {
            FailureReason::Crash(err) => {
                writeln!(out, "----------------------------------------")?;
                writeln!(out, "Test: {}\n", self.meta.item.item)?;
                err.emit_diagnostics(out, sources)
                    .expect("failed writing diagnostics");
            }
            FailureReason::ReturnedNone => {}
            FailureReason::ReturnedErr(e) => {
                writeln!(out, "----------------------------------------")?;
                writeln!(out, "Test: {}\n", self.meta.item.item)?;
                writeln!(out, "Return value: {:?}\n", e)?;
            }
        }
        Ok(())
    }
}

pub(crate) async fn do_tests(
    test_args: &crate::TestFlags,
    mut out: StandardStream,
    runtime: Arc<RuntimeContext>,
    unit: Arc<Unit>,
    sources: Sources,
    tests: Vec<(Hash, CompileMeta)>,
) -> anyhow::Result<ExitCode> {
    let mut cases = tests
        .into_iter()
        .map(|v| TestCase::from_parts(v.0, v.1))
        .collect::<Vec<_>>();

    writeln!(out, "Found {} tests...", cases.len())?;

    let start = Instant::now();
    let mut failure_count = 0;
    let mut executed_count = 0;
    for test in &mut cases {
        executed_count += 1;
        let vm = Vm::new(runtime.clone(), unit.clone());

        test.start(&mut out, test_args.quiet)?;
        let success = test.execute(unit.as_ref(), vm).await?;
        test.end(&mut out, test_args.quiet)?;
        if !success {
            failure_count += 1;
            if !test_args.no_fail_fast {
                break;
            }
        }
    }

    if test_args.quiet {
        writeln!(out)?;
    }
    let elapsed = start.elapsed();

    for case in &cases {
        case.emit_diagnostics(&mut out, &sources)?;
    }

    writeln!(out, "====")?;
    writeln!(
        out,
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
