use crate::ExitCode;
use rune::{
    termcolor::{ColorChoice, StandardStream},
    EmitDiagnostics, Sources,
};
use runestick::{
    CompileMeta, CompileMetaKind, Hash, RuntimeContext, SourceId, Span, Unit, UnitFn, Value, Vm,
    VmError, VmErrorKind,
};
use std::{collections::HashMap, io::Write, sync::Arc, time::Instant};

#[derive(Default)]
pub struct TestVisitor {
    pub test_functions: Vec<(Hash, CompileMeta)>,
}

impl rune::CompileVisitor for TestVisitor {
    fn visit_meta(&mut self, _source_id: SourceId, meta: &CompileMeta, _span: Span) {
        let type_hash = match &meta.kind {
            CompileMetaKind::Function { is_test, type_hash } if *is_test => type_hash,
            _ => return,
        };

        self.test_functions.push((*type_hash, meta.clone()));
    }
}

enum FailureReason {
    Crash(VmError),
    ReturnedNone,
    ReturnedErr(Result<Value, Value>),
}

pub(crate) async fn do_tests(
    _args: &crate::Args, // TODO: capture-output flag
    mut out: StandardStream,
    runtime: Arc<RuntimeContext>,
    unit: Arc<Unit>,
    sources: Sources,
    tests: Vec<(Hash, CompileMeta)>,
) -> anyhow::Result<ExitCode> {
    // TODO: use rune-tests capture_output to stop prints from tests from showing
    let start = Instant::now();
    let mut failures = HashMap::new();

    for test in &tests {
        write!(out, "testing {:40} ", test.1.item.item)?;
        let mut vm = Vm::new(runtime.clone(), unit.clone());

        let info = unit.lookup(test.0).ok_or_else(|| {
            VmError::from(VmErrorKind::MissingEntry {
                hash: test.0,
                item: test.1.item.item.clone(),
            })
        })?;

        let offset = match info {
            // NB: we ignore the calling convention.
            // everything is just async when called externally.
            UnitFn::Offset { offset, .. } => offset,
            _ => {
                return Err(VmError::from(VmErrorKind::MissingFunction { hash: test.0 }).into());
            }
        };

        vm.set_ip(offset);
        match vm.async_complete().await {
            Err(e) => {
                // TODO: store output here
                failures.insert(test.1.item.item.clone(), FailureReason::Crash(e));
                writeln!(out, "crashed")?;
            }
            Ok(v) => {
                if let Ok(v) = v.clone().into_result() {
                    let res = v.take().unwrap();
                    if res.is_err() {
                        failures.insert(test.1.item.item.clone(), FailureReason::ReturnedErr(res));
                        writeln!(out, "returned error")?;
                    }

                    continue;
                }
                if let Ok(v) = v.into_option() {
                    if v.borrow_ref().unwrap().is_none() {
                        failures.insert(test.1.item.item.clone(), FailureReason::ReturnedNone);
                        writeln!(out, "returned none")?;
                    }

                    continue;
                }

                writeln!(out, "passed")?;
            }
        }
    }

    let elapsed = start.elapsed();

    let failure_count = failures.len();
    for (item, error) in failures {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        match error {
            FailureReason::Crash(err) => {
                println!("----------------------------------------");
                println!("Test: {}\n", item);
                err.emit_diagnostics(&mut writer, &sources)
                    .expect("failed writing info");
            }
            FailureReason::ReturnedNone => continue,
            FailureReason::ReturnedErr(e) => {
                println!("----------------------------------------");
                println!("Test: {}\n", item);
                println!("Return value: {:?}\n", e);
            }
        }
    }

    println!("====");
    println!(
        "Ran {} tests with {} failures in {:.3} seconds",
        tests.len(),
        failure_count,
        elapsed.as_secs_f64()
    );

    if failure_count == 0 {
        Ok(ExitCode::Success)
    } else {
        Ok(ExitCode::Failure)
    }
}
