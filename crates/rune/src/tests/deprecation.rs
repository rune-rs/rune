#![cfg(feature = "doc")]

prelude!();

use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};

use crate::diagnostics::Diagnostic;

use self::diagnostics::{RuntimeDiagnosticKind, WarningDiagnosticKind};

fn create_context() -> Result<Context> {
    #[derive(Debug, rune::Any)]
    #[rune(item = ::abc)]
    pub struct TestStruct {}

    /// Creates a new TestStruct
    #[rune::function(free, path = TestStruct::new)]
    pub fn new() -> TestStruct {
        TestStruct {}
    }

    let mut module = Module::with_crate("abc")?;
    module.ty::<TestStruct>()?;
    module.function_meta(new)?;

    module
        .field_function(&Protocol::GET, "hello", |_this: &TestStruct| 1)?
        .deprecated("Deprecated get field fn")?;

    module
        .function("abc", || 1)
        .build()?
        .deprecated("Deprecated function")?;

    module
        .associated_function("test", |_this: &TestStruct| 1)?
        .deprecated("Deprecated associated fn")?;

    // Not deprecated, and must not produce any diagnostics.
    module.associated_function("ok", |_this: &TestStruct| 1)?;

    let mut context = Context::with_default_modules()?;
    context.install(module)?;
    Ok(context)
}

fn create_sources() -> Result<Sources> {
    Ok(sources! {
        entry => {
            #[test]
            pub fn main() {
                let x = abc::abc();
                let ts = abc::TestStruct::new();
                x += ts.test();
                x += ts.hello;
                x += ts.ok();
            }
        }
    })
}

/// Number of times the looping sources below call the same deprecated function.
const CALLS: usize = 3;

/// Calls a single deprecated associated function in a loop, so that the number
/// of diagnostics produced can be compared against the number of calls made.
fn create_looping_sources() -> Result<Sources> {
    Ok(sources! {
        entry => {
            #[test]
            pub fn main() {
                let ts = abc::TestStruct::new();
                let x = 0;

                for _ in 0..3 {
                    x += ts.test();
                }
            }
        }
    })
}

/// Extract the hash from a runtime deprecation diagnostic, asserting that it is
/// a runtime diagnostic rather than a compile-time one.
fn runtime_deprecation_hash(diagnostic: &Diagnostic) -> Hash {
    match diagnostic {
        Diagnostic::Runtime(w) => match w.kind() {
            RuntimeDiagnosticKind::UsedDeprecated { hash, .. } => *hash,
        },
        kind => panic!("Expected a runtime diagnostic, got: {kind:?}"),
    }
}

/// Assert that looping over a deprecated function reports it once per call.
///
/// Runtime deprecations are *not* deduplicated: [`Diagnostics::runtime_warning`]
/// pushes unconditionally, so a deprecated function invoked in a hot loop
/// produces one diagnostic per iteration rather than one per function or one
/// per call site. This test pins that behaviour down; if deduplication is ever
/// introduced, it is expected to fail and be updated deliberately.
fn assert_reported_once_per_call(context: &Context, diagnostics: &Diagnostics) {
    let mut iter = diagnostics.diagnostics().iter();

    let mut hashes = Vec::new();

    for _ in 0..CALLS {
        let diagnostic = iter
            .next()
            .expect("expected one diagnostic per call to the deprecated function");

        check_diagnostic(context, diagnostic, "Deprecated associated fn");
        hashes.push(runtime_deprecation_hash(diagnostic));
    }

    // Every diagnostic refers to the same deprecated function.
    assert!(hashes.windows(2).all(|w| w[0] == w[1]));

    // And nothing beyond one diagnostic per call is reported.
    assert!(iter.next().is_none());
}

fn check_diagnostic(context: &Context, diagnostic: &Diagnostic, msg: &str) {
    match diagnostic {
        Diagnostic::Warning(w) => match w.kind() {
            WarningDiagnosticKind::UsedDeprecated { message, .. } => {
                assert_eq!(message, msg);
            }
            kind => panic!("Unexpected warning: {kind:?}"),
        },
        Diagnostic::Runtime(w) => match w.kind() {
            RuntimeDiagnosticKind::UsedDeprecated { hash, .. } => {
                let message = context.lookup_deprecation(*hash);
                assert_eq!(message, Some(msg));
            }
        },
        kind => panic!("Unexpected diagnostics: {kind:?}"),
    };
}

#[test]
fn test_deprecation_warnings() -> Result<()> {
    let context = create_context()?;
    let runtime = Arc::try_new(context.runtime()?)?;
    let mut sources = create_sources()?;
    let mut diagnostics = Diagnostics::new();

    let unit = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build()?;

    let unit = Arc::try_new(unit)?;
    let mut vm = Vm::new(runtime, unit.clone());

    vm.call_with_diagnostics(["main"], (), &mut diagnostics)?;

    // print diagnostics - just for manual check
    if !diagnostics.is_empty() {
        diagnostics.emit_detailed(
            &mut StandardStream::stdout(ColorChoice::Auto),
            &sources,
            &unit,
            &context,
        )?;
    }

    // check that the diagnostics are properly found
    let mut iter = diagnostics.diagnostics().iter();
    check_diagnostic(&context, iter.next().unwrap(), "Deprecated function");
    check_diagnostic(&context, iter.next().unwrap(), "Deprecated associated fn");
    check_diagnostic(&context, iter.next().unwrap(), "Deprecated get field fn");
    assert!(iter.next().is_none());
    Ok(())
}

#[test]
fn test_deprecation_reported_once_per_call() -> Result<()> {
    let context = create_context()?;
    let runtime = Arc::try_new(context.runtime()?)?;
    let mut sources = create_looping_sources()?;
    let mut diagnostics = Diagnostics::new();

    let unit = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build()?;

    let unit = Arc::try_new(unit)?;
    let mut vm = Vm::new(runtime, unit);

    vm.call_with_diagnostics(["main"], (), &mut diagnostics)?;

    assert_reported_once_per_call(&context, &diagnostics);
    Ok(())
}

#[tokio::test]
async fn test_deprecation_reported_once_per_call_async() -> Result<()> {
    let context = create_context()?;
    let runtime = Arc::try_new(context.runtime()?)?;
    let mut sources = create_looping_sources()?;
    let mut diagnostics = Diagnostics::new();

    let unit = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build()?;

    let unit = Arc::try_new(unit)?;
    let vm = Vm::new(runtime, unit);

    let future = vm.send_execute(["main"], ())?;
    future.complete_with_diagnostics(&mut diagnostics).await?;

    assert_reported_once_per_call(&context, &diagnostics);
    Ok(())
}

#[tokio::test]
async fn test_deprecation_warnings_async() -> Result<()> {
    let context = create_context()?;
    let runtime = Arc::try_new(context.runtime()?)?;

    let mut sources = create_sources()?;
    let mut diagnostics = Diagnostics::new();

    let unit = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build()?;

    let unit = Arc::try_new(unit)?;
    let vm = Vm::new(runtime, unit);

    let future = vm.send_execute(["main"], ())?;
    let _ = future.complete_with_diagnostics(&mut diagnostics).await;

    // print diagnostics - just for manual check
    if !diagnostics.is_empty() {
        diagnostics.emit(&mut StandardStream::stdout(ColorChoice::Auto), &sources)?;
    }

    // check that the diagnostics are properly found
    let mut iter = diagnostics.diagnostics().iter();
    check_diagnostic(&context, iter.next().unwrap(), "Deprecated function");
    check_diagnostic(&context, iter.next().unwrap(), "Deprecated associated fn");
    check_diagnostic(&context, iter.next().unwrap(), "Deprecated get field fn");
    assert!(iter.next().is_none());
    Ok(())
}
