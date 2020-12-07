//! Test cases for rune.

pub use rune::WarningKind::*;
pub use rune::{CompileErrorKind, CompileErrorKind::*};
use rune::{Error, Errors, Sources, UnitBuilder, Warnings};
pub use rune::{ParseErrorKind, ParseErrorKind::*};
pub use rune::{QueryErrorKind, QueryErrorKind::*};
pub use rune::{ResolveErrorKind, ResolveErrorKind::*};
pub use runestick::VmErrorKind::*;
pub use runestick::{
    Bytes, CompileMeta, CompileMetaKind, ContextError, FromValue, Function, IntoComponent, Span,
    ToValue, Value, VecTuple, VmError,
};
use runestick::{Item, Source, Unit};
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
    Errors(Errors),
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
    context: &runestick::Context,
    sources: &mut Sources,
) -> Result<(Unit, Warnings), Errors> {
    let mut errors = Errors::new();
    let mut warnings = Warnings::new();

    let unit = UnitBuilder::with_default_prelude();

    if let Err(()) = rune::compile(context, sources, &unit, &mut errors, &mut warnings) {
        return Err(errors);
    }

    let unit = match unit.build() {
        Ok(unit) => unit,
        Err(error) => {
            errors.push(Error::new(0, error));
            return Err(errors);
        }
    };

    Ok((unit, warnings))
}

/// Compile the given source into a unit and collection of warnings.
pub fn compile_source(
    context: &runestick::Context,
    source: &str,
) -> Result<(Unit, Warnings), Errors> {
    let mut sources = Sources::new();
    sources.insert(Source::new("main", source.to_owned()));

    internal_compile_source(context, &mut sources)
}

/// Construct a virtual machine for the given sources.
pub fn vm(context: &runestick::Context, sources: &mut Sources) -> Result<runestick::Vm, RunError> {
    let (unit, _) = internal_compile_source(context, sources).map_err(RunError::Errors)?;
    let context = Arc::new(context.runtime());

    Ok(runestick::Vm::new(context, Arc::new(unit)))
}

/// Construct a virtual machine for the given source.
pub fn vm_with_source(
    context: &runestick::Context,
    source: &str,
) -> Result<runestick::Vm, RunError> {
    let mut sources = Sources::new();
    sources.insert(Source::new("main", source.to_owned()));

    vm(context, &mut sources)
}

/// Call the specified function in the given script.
async fn internal_run_async<N, A, T>(
    context: &Arc<runestick::Context>,
    sources: &mut Sources,
    function: N,
    args: A,
) -> Result<T, RunError>
where
    N: IntoIterator,
    N::Item: IntoComponent,
    A: runestick::Args,
    T: FromValue,
{
    let vm = vm(context, sources)?;

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
    context: &Arc<runestick::Context>,
    sources: &mut Sources,
    function: N,
    args: A,
) -> Result<T, RunError>
where
    N: IntoIterator,
    N::Item: IntoComponent,
    A: runestick::Args,
    T: FromValue,
{
    ::futures_executor::block_on(internal_run_async(context, sources, function, args))
}

/// Call the specified function in the given script sources.
#[cfg(not(feature = "futures-executor"))]
fn internal_run<N, A, T>(
    context: &Arc<runestick::Context>,
    sources: &mut Sources,
    function: N,
    args: A,
) -> Result<T, RunError>
where
    N: IntoIterator,
    N::Item: IntoComponent,
    A: runestick::Args,
    T: FromValue,
{
    let vm = vm(context, sources)?;

    let output = vm
        .execute(&Item::with_item(function), args)
        .map_err(RunError::VmError)?
        .complete()
        .map_err(RunError::VmError)?;

    T::from_value(output).map_err(RunError::VmError)
}

/// Run the given source with diagnostics being printed to stderr.
pub fn run_with_diagnostics<N, A, T>(
    context: &Arc<runestick::Context>,
    source: &str,
    function: N,
    args: A,
) -> Result<T, RunError>
where
    N: IntoIterator,
    N::Item: IntoComponent,
    A: runestick::Args,
    T: runestick::FromValue,
{
    use rune::diagnostics::EmitDiagnostics as _;

    let mut sources = Sources::new();
    sources.insert(Source::new("main", source.to_owned()));

    let e = match internal_run(context, &mut sources, function, args) {
        Ok(value) => return Ok(value),
        Err(e) => e,
    };

    let mut writer = rune::termcolor::StandardStream::stdout(rune::termcolor::ColorChoice::Never);

    match &e {
        RunError::Errors(e) => {
            e.emit_diagnostics(&mut writer, &sources)
                .expect("emit diagnostics");
        }
        RunError::VmError(e) => {
            e.emit_diagnostics(&mut writer, &sources)
                .expect("emit diagnostics");
        }
    }

    Err(e)
}

/// Call the specified function in the given script.
pub fn run<N, A, T>(
    context: &Arc<runestick::Context>,
    source: &str,
    function: N,
    args: A,
) -> Result<T, RunError>
where
    N: IntoIterator,
    N::Item: IntoComponent,
    A: runestick::Args,
    T: runestick::FromValue,
{
    let mut sources = Sources::new();
    sources.insert(Source::new("main", source.to_owned()));

    internal_run(context, &mut sources, function, args)
}

/// Helper function to construct a context and unit from a Rune source for
/// testing purposes.
///
/// This is primarily used in examples.
pub fn build(
    context: &runestick::Context,
    source: &str,
) -> runestick::Result<Arc<runestick::Unit>> {
    let options = rune::Options::default();
    let mut sources = rune::Sources::new();
    sources.insert(runestick::Source::new("source", source));

    let mut warnings = rune::Warnings::new();
    let mut errors = rune::Errors::new();

    let unit = match rune::load_sources(
        &*context,
        &options,
        &mut sources,
        &mut errors,
        &mut warnings,
    ) {
        Ok(unit) => unit,
        Err(error) => {
            let mut writer =
                rune::termcolor::StandardStream::stderr(rune::termcolor::ColorChoice::Always);
            rune::EmitDiagnostics::emit_diagnostics(&errors, &mut writer, &sources)?;
            return Err(error.into());
        }
    };

    if !warnings.is_empty() {
        let mut writer =
            rune::termcolor::StandardStream::stderr(rune::termcolor::ColorChoice::Always);
        rune::EmitDiagnostics::emit_diagnostics(&warnings, &mut writer, &sources)?;
    }

    Ok(std::sync::Arc::new(unit))
}

/// Construct a rune virtual machine from the given program.
///
/// # Examples
///
/// ```rust
/// use rune_tests::*;
/// use runestick::Value;
///
/// # fn main() {
/// let vm = rune_tests::rune_vm!(pub fn main() { true || false });
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
/// use runestick::Value;
///
/// # fn main() {
/// let vm = rune_tests::rune_vm!(pub fn main() { true || false });
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
/// use runestick::Module;
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
        let context = std::sync::Arc::new(rune_modules::default_context().unwrap());
        let errors = $crate::compile_source(&context, &$source).unwrap_err();
        let err = errors.into_iter().next().expect("expected one error");

        let e = match err.into_kind() {
            rune::ErrorKind::ParseError(e) => (e),
            kind => {
                panic!(
                    "expected parse error `{}` but was `{:?}`",
                    stringify!($pat),
                    kind
                );
            }
        };

        let $span = rune::Spanned::span(&e);

        match e.into_kind() {
            $pat => $cond,
            kind => {
                panic!("expected error `{}` but was `{:?}`", stringify!($pat), kind);
            }
        }
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
        let context = $crate::macros::rune_modules::default_context().unwrap();
        let e = $crate::compile_source(&context, $source).unwrap_err();
        let e = e.into_iter().next().expect("expected one error");

        let e = match e.into_kind() {
            rune::ErrorKind::CompileError(e) => (e),
            kind => {
                panic!(
                    "expected parse error `{}` but was `{:?}`",
                    stringify!($pat),
                    kind
                );
            }
        };

        let $span = rune::Spanned::span(&e);

        match e.into_kind() {
            $pat => $cond,
            kind => {
                panic!("expected error `{}` but was `{:?}`", stringify!($pat), kind);
            }
        }
    }};
}

/// Assert that the given rune program parses, but raises the specified set of
/// warnings.
#[macro_export]
macro_rules! assert_warnings {
    ($source:expr $(, $pat:pat => $cond:expr)*) => {{
        let context = $crate::macros::rune_modules::default_context().unwrap();
        let (_, warnings) = $crate::compile_source(&context, $source).expect("source should compile");
        assert!(!warnings.is_empty(), "no warnings produced");

        let mut it = warnings.into_iter();

        $(
            let warning = it.next().expect("expected a warning");

            match warning.kind {
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
