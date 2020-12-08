use crate::ExitCode;
use rune::{
    termcolor::{ColorChoice, StandardStream},
    EmitDiagnostics, Sources,
};
use runestick::{
    CompileMeta, CompileMetaKind, Hash, RuntimeContext, SourceId, Span, Unit, UnitFn, Vm, VmError,
    VmErrorKind,
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
                failures.insert(test.1.item.item.clone(), e);
                writeln!(out, "failed")?;
            }
            Ok(_) => {
                writeln!(out, "ok.")?;
            }
        }
    }

    let elapsed = start.elapsed();

    for (item, error) in &failures {
        println!("----------------------------------------");
        println!("Test: {}\n", item);

        let mut writer = StandardStream::stderr(ColorChoice::Always);
        error
            .emit_diagnostics(&mut writer, &sources)
            .expect("failed writing info");
    }

    println!("====");
    println!(
        "Ran {} tests with {} failures in {:.3} seconds",
        tests.len(),
        failures.len(),
        elapsed.as_secs_f64()
    );

    if failures.is_empty() {
        Ok(ExitCode::Success)
    } else {
        Ok(ExitCode::Failure)
    }
}
