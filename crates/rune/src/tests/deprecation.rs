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
            }
        }
    })
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

    Ok(())
}
