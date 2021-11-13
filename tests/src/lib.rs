//! Test cases for rune.
#![allow(dead_code)]

use rune::compile::{IntoComponent, Item};
use rune::runtime::{Args, VmError};
use rune::{termcolor, Context, Diagnostics, FromValue, Source, Sources, Unit, Vm};
use std::sync::Arc;
use thiserror::Error;

pub mod capture_output;

/// Macro internals.
#[doc(hidden)]
pub mod macros {
    pub use ::rune_modules;
}

/// An error that can be raised during testing.
#[derive(Debug, Error)]
pub enum RunError {
    /// A load error was raised during testing.
    #[error("load errors")]
    Diagnostics(Diagnostics),
    /// A virtual machine error was raised during testing.
    #[error("vm error")]
    VmError(#[source] VmError),
}

impl RunError {
    /// Unpack into a vm error or panic with the given message.
    pub fn expect_vm_error(self, msg: &str) -> VmError {
        match self {
            Self::VmError(error) => error,
            _ => panic!("{}", msg),
        }
    }
}

fn internal_compile_source(
    context: &Context,
    sources: &mut Sources,
) -> Result<(Unit, Diagnostics), Diagnostics> {
    let mut diagnostics = Diagnostics::new();

    let unit = match rune::prepare(sources)
        .with_context(context)
        .with_diagnostics(&mut diagnostics)
        .build()
    {
        Ok(unit) => unit,
        Err(..) => return Err(diagnostics),
    };

    Ok((unit, diagnostics))
}

/// Compile the given source into a unit and collection of warnings.
pub fn compile_source(context: &Context, source: &str) -> Result<(Unit, Diagnostics), Diagnostics> {
    let mut sources = Sources::new();
    sources.insert(Source::new("main", source.to_owned()));

    internal_compile_source(context, &mut sources)
}

/// Construct a virtual machine for the given sources.
pub fn vm(context: &Context, sources: &mut Sources) -> Result<Vm, RunError> {
    let (unit, _) = internal_compile_source(context, sources).map_err(RunError::Diagnostics)?;
    let context = Arc::new(context.runtime());

    Ok(Vm::new(context, Arc::new(unit)))
}

/// Construct a virtual machine for the given source.
pub fn vm_with_source(context: &Context, source: &str) -> Result<Vm, RunError> {
    let mut sources = Sources::new();
    sources.insert(Source::new("main", source.to_owned()));

    vm(context, &mut sources)
}

/// Call the specified function in the given script.
async fn internal_run_async<N, A, T>(
    context: &Arc<Context>,
    sources: &mut Sources,
    function: N,
    args: A,
) -> Result<T, RunError>
where
    N: IntoIterator,
    N::Item: IntoComponent,
    A: Args,
    T: FromValue,
{
    let mut vm = vm(context, sources)?;

    let output = vm
        .execute(&Item::with_item(function), args)
        .map_err(RunError::VmError)?
        .async_complete()
        .await
        .map_err(RunError::VmError)?;

    T::from_value(output).map_err(RunError::VmError)
}

/// Call the specified function in the given script sources.
#[cfg(feature = "futures-executor")]
fn internal_run<N, A, T>(
    context: &Arc<Context>,
    sources: &mut Sources,
    function: N,
    args: A,
) -> Result<T, RunError>
where
    N: IntoIterator,
    N::Item: IntoComponent,
    A: Args,
    T: FromValue,
{
    ::futures_executor::block_on(internal_run_async(context, sources, function, args))
}

/// Call the specified function in the given script sources.
#[cfg(not(feature = "futures-executor"))]
fn internal_run<N, A, T>(
    context: &Arc<Context>,
    sources: &mut Sources,
    function: N,
    args: A,
) -> Result<T, RunError>
where
    N: IntoIterator,
    N::Item: IntoComponent,
    A: Args,
    T: FromValue,
{
    let mut vm = vm(context, sources)?;

    let output = vm
        .execute(&Item::with_item(function), args)
        .map_err(RunError::VmError)?
        .complete()
        .map_err(RunError::VmError)?;

    T::from_value(output).map_err(RunError::VmError)
}

/// Run the given source with diagnostics being printed to stderr.
pub fn run_with_diagnostics<N, A, T>(
    context: &Arc<Context>,
    source: &str,
    function: N,
    args: A,
) -> Result<T, RunError>
where
    N: IntoIterator,
    N::Item: IntoComponent,
    A: Args,
    T: FromValue,
{
    let mut sources = Sources::new();
    sources.insert(Source::new("main", source.to_owned()));

    let e = match internal_run(context, &mut sources, function, args) {
        Ok(value) => return Ok(value),
        Err(e) => e,
    };

    let mut writer = termcolor::StandardStream::stdout(termcolor::ColorChoice::Never);

    match &e {
        RunError::Diagnostics(diagnostics) => {
            diagnostics
                .emit(&mut writer, &sources)
                .expect("emit diagnostics");
        }
        RunError::VmError(e) => {
            e.emit(&mut writer, &sources).expect("emit diagnostics");
        }
    }

    Err(e)
}

/// Call the specified function in the given script.
pub fn run<N, A, T>(
    context: &Arc<Context>,
    source: &str,
    function: N,
    args: A,
) -> Result<T, RunError>
where
    N: IntoIterator,
    N::Item: IntoComponent,
    A: Args,
    T: FromValue,
{
    let mut sources = Sources::new();
    sources.insert(Source::new("main", source.to_owned()));

    internal_run(context, &mut sources, function, args)
}

/// Helper function to construct a context and unit from a Rune source for
/// testing purposes.
///
/// This is primarily used in examples.
pub fn build(context: &Context, source: &str) -> rune::Result<Arc<Unit>> {
    let mut sources = Sources::new();
    sources.insert(Source::new("source", source));

    let mut diagnostics = Diagnostics::new();

    let result = rune::prepare(&mut sources)
        .with_context(context)
        .with_diagnostics(&mut diagnostics)
        .build();

    if !diagnostics.is_empty() {
        let mut writer = termcolor::StandardStream::stderr(termcolor::ColorChoice::Always);
        diagnostics.emit(&mut writer, &sources)?;
    }

    Ok(Arc::new(result?))
}

/// Construct a rune virtual machine from the given program.
///
/// # Examples
///
/// ```rust
/// use rune_tests::*;
/// use rune::Value;
///
/// # fn main() {
/// let mut vm = rune_tests::rune_vm!(pub fn main() { true || false });
/// let result = vm.execute(&["main"], ()).unwrap().complete().unwrap();
/// assert_eq!(result.into_bool().unwrap(), true);
/// # }
#[macro_export]
macro_rules! rune_vm {
    ($($tt:tt)*) => {{
        let context = $crate::macros::rune_modules::default_context().expect("failed to build context");
        let context = std::sync::Arc::new(context);
        $crate::vm_with_source(&context, stringify!($($tt)*)).expect("program to compile successfully")
    }};
}

/// Construct a rune virtual machine from the given program which will capture
/// all output into a buffer, which can be retrieved from
/// `rune_tests::capture_output::drain_output()`
///
/// # Examples
///
/// ```rust
/// use rune_tests::*;
/// use rune::Value;
///
/// # fn main() {
/// let mut vm = rune_tests::rune_vm!(pub fn main() { true || false });
/// let result = vm.execute(&["main"], ()).unwrap().complete().unwrap();
/// assert_eq!(result.into_bool().unwrap(), true);
/// # }
#[macro_export]
macro_rules! rune_vm_capture {
    ($($tt:tt)*) => {{
        let mut context = $crate::macros::rune_modules::with_config(false).expect("failed to build context");
        context.install(&$crate::capture_output::output_redirect_module()?)?;
        let context = std::sync::Arc::new(context);
        $crate::vm_with_source(&context, stringify!($($tt)*)).expect("program to compile successfully")
    }};
}

/// Same as [rune_s!] macro, except it takes a Rust token tree. This works
/// fairly well because Rust and Rune has very similar token trees.
///
/// # Examples
///
/// ```rust
/// use rune_tests::*;
///
/// # fn main() {
/// assert_eq! {
///     rune_tests::rune!(bool => pub fn main() { true || false }),
///     true,
/// };
/// # }
#[macro_export]
macro_rules! rune {
    ($ty:ty => $($tt:tt)*) => {{
        let context = $crate::macros::rune_modules::default_context().expect("failed to build context");
        let context = std::sync::Arc::new(context);

        $crate::run_with_diagnostics::<_, (), $ty>(&context, stringify!($($tt)*), &["main"], ())
            .expect("program to run successfully")
    }};
}

/// Run the given program and return the expected type from it.
///
/// # Examples
///
/// ```rust
/// use rune_tests::*;
///
/// # fn main() {
/// assert_eq! {
///     rune_tests::rune_s!(bool => "pub fn main() { true || false }"),
///     true,
/// };
/// # }
/// ```
#[macro_export]
macro_rules! rune_s {
    ($ty:ty => $source:expr) => {{
        let context =
            $crate::macros::rune_modules::default_context().expect("failed to build context");
        let context = std::sync::Arc::new(context);

        $crate::run_with_diagnostics::<_, (), $ty>(&context, $source, &["main"], ())
            .expect("program to run successfully")
    }};
}

/// Same as [rune!] macro, except it takes an external context, allowing testing
/// of native Rust data. This also accepts a tuple of arguments in the second
/// position, to pass native objects as arguments to the script.
///
/// # Examples
///
/// ```rust
/// use rune_tests::*;
/// use rune::Module;
/// fn get_native_module() -> Module {
///     Module::new()
/// }
///
/// # fn main() {
/// assert_eq! {
///     rune_tests::rune_n!(get_native_module(), (), bool => pub fn main() { true || false }),
///     true,
/// };
/// # }
#[macro_export]
macro_rules! rune_n {
    ($module:expr, $args:expr, $ty:ty => $($tt:tt)*) => {{
        let mut context = $crate::macros::rune_modules::default_context().expect("failed to build context");
        context.install(&$module).expect("failed to install native module");
        let context = std::sync::Arc::new(context);

        $crate::run_with_diagnostics::<_, _, $ty>(&context, stringify!($($tt)*), &["main"], $args)
            .expect("program to run successfully")
    }};
}

/// Assert that the given parse error happens with the given rune program.
#[macro_export]
macro_rules! assert_parse_error {
    ($source:expr, $span:ident, $pat:pat => $cond:expr) => {{
        $crate::assert_errors!($source, $span, ParseError($pat) => $cond)
    }};
}

/// Assert that the given vm error happens with the given rune program.
#[macro_export]
macro_rules! assert_vm_error {
    // Second variant which allows for specifyinga type.
    ($source:expr, $pat:pat => $cond:block) => {
        assert_vm_error!(() => $source, $pat => $cond)
    };

    // Second variant which allows for specifyinga type.
    ($ty:ty => $source:expr, $pat:pat => $cond:block) => {{
        let context = std::sync::Arc::new(rune_modules::default_context().unwrap());
        let e = $crate::run::<_, _, $ty>(&context, $source, &["main"], ()).unwrap_err();

        let (e, _) = match e {
            $crate::RunError::VmError(e) => e.into_unwound(),
            actual => {
                panic!("expected vm error `{}` but was `{:?}`", stringify!($pat), actual);
            }
        };

        match e.into_kind() {
            $pat => $cond,
            actual => {
                panic!("expected error `{}` but was `{:?}`", stringify!($pat), actual);
            }
        }
    }};
}

/// Assert that the given rune program parses.
#[macro_export]
macro_rules! assert_parse {
    ($source:expr) => {{
        let context = $crate::macros::rune_modules::default_context().unwrap();
        $crate::compile_source(&context, $source).unwrap()
    }};
}

/// Assert that the given rune program raises a compile error.
#[macro_export]
macro_rules! assert_compile_error {
    ($source:expr, $span:ident, $pat:pat => $cond:expr) => {{
        $crate::assert_errors!($source, $span, CompileError($pat) => $cond)
    }};
}

/// Assert that the given rune program raises a query error.
#[macro_export]
macro_rules! assert_errors {
    ($source:expr, $span:ident, $($variant:ident($pat:pat) => $cond:expr),+ $(,)?) => {{
        let context = $crate::macros::rune_modules::default_context().unwrap();
        let e = $crate::compile_source(&context, $source).unwrap_err();
        assert!(e.has_error(), "expected at least one error");

        let mut it = e.into_diagnostics().into_iter();

        $(
            let e = match it.next().expect("expected error") {
                rune::diagnostics::Diagnostic::Fatal(e) => e,
                kind => {
                    panic!(
                        "expected diagnostic error `{}` but was `{:?}`",
                        stringify!($pat),
                        kind
                    );
                }
            };

            let e = match e.into_kind() {
                rune::diagnostics::FatalDiagnosticKind::$variant(e) => (e),
                kind => {
                    panic!("expected error of variant `{}` but was `{:?}`", stringify!($variant), kind);
                }
            };

            let $span = rune::ast::Spanned::span(&e);

            match e.into_kind() {
                $pat => $cond,
                kind => {
                    panic!("expected error `{}` but was `{:?}`", stringify!($pat), kind);
                }
            }
        )+
    }};
}

/// Assert that the given rune program parses, but raises the specified set of
/// warnings.
#[macro_export]
macro_rules! assert_warnings {
    ($source:expr $(, $pat:pat => $cond:expr)*) => {{
        let context = $crate::macros::rune_modules::default_context().unwrap();
        let (_, diagnostics) = $crate::compile_source(&context, $source).expect("source should compile");
        assert!(diagnostics.has_warning(), "no warnings produced");

        let mut it = diagnostics.into_diagnostics().into_iter();

        $(
            let warning = it.next().expect("expected a warning");

            let warning = match warning {
                rune::diagnostics::Diagnostic::Warning(warning) => warning,
                kind => {
                    panic!(
                        "expected diagnostic warning `{}` but was `{:?}`",
                        stringify!($pat),
                        kind
                    );
                }
            };

            match warning.into_kind() {
                $pat => ($cond),
                warning => {
                    panic!("expected warning `{}` but was `{:?}`", stringify!($pat), warning);
                }
            }
        )*

        assert!(it.next().is_none(), "there should be no more warnings");
    }};
}

/// Assert that the given value matches the provided pattern.
#[macro_export]
macro_rules! assert_matches {
    ($value:expr, $pat:pat) => {
        match $value {
            $pat => (),
            other => panic!("expected {}, but was {:?}", stringify!($pat), other),
        }
    };
}
