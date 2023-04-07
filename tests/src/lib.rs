//! Test cases for rune.
#![allow(dead_code)]

pub mod prelude {
    pub use crate::{
        assert_compile_error, assert_errors, assert_matches, assert_parse, assert_parse_error,
        assert_vm_error, assert_warnings, run,
    };
    pub use crate::{rune, rune_n, rune_s};
    pub use ::rune_modules as modules;
    pub use futures_executor::block_on;
    pub use rune::ast;
    pub use rune::compile::{self, CompileErrorKind, Item, Location, Named};
    pub use rune::diagnostics;
    pub use rune::macros;
    pub use rune::parse::{self, ParseErrorKind, ResolveErrorKind};
    pub use rune::query::QueryErrorKind;
    pub use rune::runtime::{
        self, AnyObj, AnyTypeInfo, Bytes, Function, MaybeTypeOf, Object, Protocol, RawRef, RawStr,
        Shared, Stack, Tuple, TypeInfo, TypeOf, UnsafeFromValue, VecTuple, VmErrorKind, VmResult,
    };
    pub use rune::{
        from_value, prepare, sources, span, to_value, vm_try, Any, Context, ContextError,
        Diagnostics, FromValue, Hash, InstallWith, Module, Result, Source, Sources, ToValue, Value,
        Vm,
    };
}

use rune::compile::{IntoComponent, ItemBuf};
use rune::runtime::{Args, VmError, VmResult};
use rune::{termcolor, BuildError, Context, Diagnostics, FromValue, Source, Sources, Unit, Vm};
use std::sync::Arc;
use thiserror::Error;

/// An error that can be raised during testing.
#[derive(Debug, Error)]
pub enum RunError {
    /// A load error was raised during testing.
    #[error("build error")]
    BuildError(BuildError),
    /// A virtual machine error was raised during testing.
    #[error("vm error: {0}")]
    VmError(VmError),
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

/// Compile the given source into a unit and collection of warnings.
#[doc(hidden)]
pub fn compile_helper(source: &str, diagnostics: &mut Diagnostics) -> Result<Unit, BuildError> {
    let context = crate::prelude::modules::default_context().expect("setting up default modules");

    let mut sources = Sources::new();
    sources.insert(Source::new("main", source));

    let unit = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(diagnostics)
        .build()?;

    Ok(unit)
}

/// Construct a virtual machine for the given sources.
#[doc(hidden)]
pub fn vm(
    context: &Context,
    sources: &mut Sources,
    diagnostics: &mut Diagnostics,
) -> Result<Vm, RunError> {
    let unit = rune::prepare(sources)
        .with_context(context)
        .with_diagnostics(diagnostics)
        .build()
        .map_err(RunError::BuildError)?;

    let context = Arc::new(context.runtime());
    Ok(Vm::new(context, Arc::new(unit)))
}

/// Call the specified function in the given script sources.
#[doc(hidden)]
pub fn run_helper<N, A, T>(
    context: &Context,
    sources: &mut Sources,
    diagnostics: &mut Diagnostics,
    function: N,
    args: A,
) -> Result<T, RunError>
where
    N: IntoIterator,
    N::Item: IntoComponent,
    A: Args,
    T: FromValue,
{
    ::futures_executor::block_on(async move {
        let mut vm = vm(context, sources, diagnostics)?;

        let mut execute = vm
            .execute(&ItemBuf::with_item(function), args)
            .map_err(RunError::VmError)?;
        let output = execute
            .async_complete()
            .await
            .into_result()
            .map_err(RunError::VmError)?;

        match T::from_value(output) {
            VmResult::Ok(output) => Ok(output),
            VmResult::Err(err) => Err(RunError::VmError(err)),
        }
    })
}

#[doc(hidden)]
pub fn sources(source: &str) -> Sources {
    let mut sources = Sources::new();
    sources.insert(Source::new("main", source));
    sources
}

/// Run the given source with diagnostics being printed to stderr.
pub fn run<N, A, T>(context: &Context, source: &str, function: N, args: A) -> Result<T, RunError>
where
    N: IntoIterator,
    N::Item: IntoComponent,
    A: Args,
    T: FromValue,
{
    let mut sources = Sources::new();
    sources.insert(Source::new("main", source));

    let mut diagnostics = Default::default();

    let e = match run_helper(context, &mut sources, &mut diagnostics, function, args) {
        Ok(value) => return Ok(value),
        Err(e) => e,
    };

    let mut writer = termcolor::StandardStream::stdout(termcolor::ColorChoice::Never);

    match &e {
        RunError::BuildError(..) => {
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
/// ```
/// use rune_tests::prelude::*;
/// use rune::Value;
///
/// let mut vm = rune_tests::rune_vm!(pub fn main() { true || false });
/// let result = vm.execute(["main"], ()).unwrap().complete().unwrap();
/// assert_eq!(result.into_bool().unwrap(), true);
/// ```
#[macro_export]
macro_rules! rune_vm {
    ($($tt:tt)*) => {{
        let context = $crate::prelude::modules::default_context().expect("failed to build context");
        let mut diagnostics = Default::default();
        let mut sources = $crate::sources(stringify!($($tt)*));
        $crate::vm(&context, &mut sources, &mut diagnostics).expect("program to compile successfully")
    }};
}

/// Construct a rune virtual machine from the given program which will capture
/// all output into a buffer, which can be retrieved from
/// `rune_tests::capture_output::drain_output()`
///
/// # Examples
///
/// ```
/// use rune_tests::prelude::*;
/// use rune::Value;
///
/// let mut vm = rune_tests::rune_vm!(pub fn main() { true || false });
/// let result = vm.execute(["main"], ()).unwrap().complete().unwrap();
/// assert_eq!(result.into_bool().unwrap(), true);
/// ```
#[macro_export]
macro_rules! rune_vm_capture {
    ($($tt:tt)*) => {{
        let mut context = $crate::prelude::modules::with_config(false)?;
        let io = $crate::prelude::modules::capture_io::CaptureIo::new();
        let m = $crate::prelude::modules::capture_io::module(&io)?;
        context.install(m)?;
        let mut sources = $crate::sources(stringify!($($tt)*));
        let mut diagnostics = Default::default();
        let vm = $crate::vm(&context, &mut sources, &mut diagnostics)?;
        (vm, io)
    }};
}

/// Same as [rune_s!] macro, except it takes a Rust token tree. This works
/// fairly well because Rust and Rune has very similar token trees.
///
/// # Examples
///
/// ```
/// use rune_tests::prelude::*;
///
/// let out: bool = rune_tests::rune!(pub fn main() { true || false });
/// assert_eq!(out, true);
/// ```
#[macro_export]
macro_rules! rune {
    ($($tt:tt)*) => {{
        let context = $crate::prelude::modules::default_context().expect("failed to build context");
        $crate::run(&context, stringify!($($tt)*), ["main"], ()).expect("program to run successfully")
    }};
}

/// Run the given program and return the expected type from it.
///
/// # Examples
///
/// ```
/// use rune_tests::prelude::*;
///
/// let out: bool = rune_tests::rune_s!("pub fn main() { true || false }");
/// assert_eq!(out, true);
/// ```
#[macro_export]
macro_rules! rune_s {
    ($source:expr) => {{
        let context = $crate::prelude::modules::default_context().expect("failed to build context");
        $crate::run(&context, $source, ["main"], ()).expect("program to run successfully")
    }};
}

/// Same as [rune!] macro, except it takes an external context, allowing testing
/// of native Rust data. This also accepts a tuple of arguments in the second
/// position, to pass native objects as arguments to the script.
///
/// # Examples
///
/// ```
/// use rune_tests::prelude::*;
/// use rune::Module;
///
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
/// ```
#[macro_export]
macro_rules! rune_n {
    ($module:expr, $args:expr, $ty:ty => $($tt:tt)*) => {{
        let mut context = $crate::prelude::modules::default_context().expect("failed to build context");
        context.install($module).expect("failed to install native module");
        $crate::run::<_, _, $ty>(&context, stringify!($($tt)*), ["main"], $args).expect("program to run successfully")
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
        let context = $crate::prelude::modules::default_context().unwrap();
        let mut diagnostics = Default::default();

        let mut sources = $crate::sources($source);
        let e = match $crate::run_helper::<_, _, $ty>(&context, &mut sources, &mut diagnostics, ["main"], ()) {
            Err(e) => e,
            Ok(value) => {
                panic!("expected error but program completed with: {:?}", value);
            }
        };

        let e = match e {
            $crate::RunError::VmError(e) => e,
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
        let mut diagnostics = Default::default();
        $crate::compile_helper($source, &mut diagnostics).unwrap()
    }};
}

/// Assert that the given rune program raises a compile error.
#[macro_export]
macro_rules! assert_compile_error {
    ($source:expr, $span:ident, $pat:pat => $cond:expr) => {{
        $crate::assert_errors!($source, $span, CompileError($pat) => $cond)
    }};
}

/// Assert that the given parse error happens with the given rune program.
#[macro_export]
macro_rules! assert_parse_error {
    ($source:expr, $span:ident, $pat:pat => $cond:expr) => {{
        $crate::assert_errors!($source, $span, ParseError($pat) => $cond)
    }};
}

/// Assert that the given rune program raises a query error.
#[macro_export]
macro_rules! assert_errors {
    ($source:expr, $span:ident, $($variant:ident($pat:pat) => $cond:expr),+ $(,)?) => {{
        let mut diagnostics = Default::default();
        let _ = $crate::compile_helper($source, &mut diagnostics).unwrap_err();

        let mut it = diagnostics.into_diagnostics().into_iter();

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
        let mut diagnostics = Default::default();
        let _ = $crate::compile_helper($source, &mut diagnostics).expect("source should compile");
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
