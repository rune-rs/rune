//! <div align="center">
//!     <img alt="Rune Logo" src="https://raw.githubusercontent.com/rune-rs/rune/master/assets/icon.png" />
//! </div>
//!
//! <br>
//!
//! <div align="center">
//! <a href="https://rune-rs.github.io/rune/">
//!     <b>Read the Book 📖</b>
//! </a>
//! </div>
//!
//! <br>
//!
//! <div align="center">
//! <a href="https://github.com/rune-rs/rune/actions">
//!     <img alt="Build Status" src="https://github.com/rune-rs/rune/workflows/Build/badge.svg">
//! </a>
//!
//! <a href="https://github.com/rune-rs/rune/actions">
//!     <img alt="Book Status" src="https://github.com/rune-rs/rune/workflows/Book/badge.svg">
//! </a>
//!
//! <a href="https://crates.io/crates/rune">
//!     <img alt="crates.io" src="https://img.shields.io/crates/v/rune.svg">
//! </a>
//!
//! <a href="https://docs.rs/rune">
//!     <img alt="docs.rs" src="https://docs.rs/rune/badge.svg">
//! </a>
//!
//! <a href="https://discord.gg/v5AeNkT">
//!     <img alt="Chat on Discord" src="https://img.shields.io/discord/558644981137670144.svg?logo=discord&style=flat-square">
//! </a>
//! </div>
//!
//! <br>
//!
//! A crate used to simplify testing of small Rune programs.
//!
//! This is a crate used with the [Rune language].
//!
//! [Rune Language]: https://github.com/rune-rs/rune

pub use futures_executor::block_on;
pub use rune::CompileError::*;
pub use rune::ParseError::*;
use rune::Sources;
use rune::UnitBuilder;
pub use rune::WarningKind::*;
use rune::{Errors, Warnings};
pub use runestick::VmErrorKind::*;
pub use runestick::{CompileMeta, CompileMetaKind, Function, IntoComponent, Span, Value};
pub use runestick::{ContextError, VmError};
use runestick::{Item, Source, Unit};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RunError {
    #[error("load errors")]
    Errors(Errors),
    #[error("vm error")]
    VmError(#[source] VmError),
    #[error("context error")]
    ContextError(#[source] ContextError),
}

/// Compile the given source into a unit and collection of warnings.
pub fn compile_source(
    context: &runestick::Context,
    source: &str,
) -> Result<(Unit, Warnings), Errors> {
    let mut errors = Errors::new();
    let mut warnings = Warnings::new();
    let mut sources = Sources::new();
    sources.insert(Source::new("main", source.to_owned()));
    let unit = Rc::new(RefCell::new(UnitBuilder::with_default_prelude()));

    if let Err(()) = rune::compile(context, &mut sources, &unit, &mut errors, &mut warnings) {
        return Err(errors);
    }

    let unit = Rc::try_unwrap(unit).unwrap().into_inner();
    Ok((unit.into_unit(), warnings))
}

/// Call the specified function in the given script.
pub async fn run_async<N, A, T>(function: N, args: A, source: &str) -> Result<T, RunError>
where
    N: IntoIterator,
    N::Item: IntoComponent,
    A: runestick::Args,
    T: runestick::FromValue,
{
    let context = runestick::Context::with_default_modules().map_err(RunError::ContextError)?;
    let (unit, _) = compile_source(&context, &source).map_err(RunError::Errors)?;

    let vm = runestick::Vm::new(Arc::new(context), Arc::new(unit));

    let output = vm
        .execute(&Item::of(function), args)
        .map_err(RunError::VmError)?
        .async_complete()
        .await
        .map_err(RunError::VmError)?;

    T::from_value(output).map_err(RunError::VmError)
}

/// Call the specified function in the given script.
pub fn run<N, A, T>(function: N, args: A, source: &str) -> Result<T, RunError>
where
    N: IntoIterator,
    N::Item: IntoComponent,
    A: runestick::Args,
    T: runestick::FromValue,
{
    block_on(run_async(function, args, source))
}

/// Run the given program and return the expected type from it.
///
/// # Examples
///
/// ```rust
/// use rune_testing::*;
///
/// # fn main() {
/// assert_eq! {
///     rune!(bool => r#"fn main() { true || false }"#),
///     true,
/// };
/// # }
/// ```
#[macro_export]
macro_rules! rune {
    ($ty:ty => $source:expr) => {
        $crate::run::<_, (), $ty>(&["main"], (), $source).expect("program to run successfully")
    };
}

/// Assert that the given parse error happens with the given rune program.
///
/// # Examples
///
/// ```rust
/// use rune_testing::*;
///
/// # fn main() {
/// assert_parse_error! {
///     r#"fn main() { 0 < 10 >= 10 }"#,
///     PrecedenceGroupRequired { span } => {
///         assert_eq!(span, Span::new(12, 18));
///     }
/// };
/// # }
/// ```
#[macro_export]
macro_rules! assert_parse_error {
    ($source:expr, $pat:pat => $cond:expr) => {{
        let context = runestick::Context::with_default_modules().unwrap();
        let errors = $crate::compile_source(&context, &$source).unwrap_err();
        let err = errors.into_iter().next().expect("expected one error");

        match err.into_kind() {
            rune::LoadErrorKind::ParseError($pat) => ($cond),
            kind => {
                panic!("expected error `{}` but was `{:?}`", stringify!($pat), kind);
            }
        }
    }};
}

/// Assert that the given vm error happens with the given rune program.
///
/// # Examples
///
/// ```rust
/// use rune_testing::*;
///
/// # fn main() {
/// assert_vm_error!(
///     r#"
///     fn main() {
///         let a = 9223372036854775807;
///         let b = 2;
///         a += b;
///     }
///     "#,
///     Overflow => {}
/// );
/// # }
/// ```
#[macro_export]
macro_rules! assert_vm_error {
    // Second variant which allows for specifyinga type.
    ($source:expr, $pat:pat => $cond:block) => {
        $crate::assert_vm_error!(() => $source, $pat => $cond)
    };

    // Second variant which allows for specifyinga type.
    ($ty:ty => $source:expr, $pat:pat => $cond:block) => {{
        let e = $crate::run::<_, _, $ty>(&["main"], (), $source).unwrap_err();

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
///
/// # Examples
///
/// ```rust
/// use rune_testing::*;
///
/// # fn main() {
/// assert_parse!(r#"fn main() { (0 < 10) >= 10 }"#);
/// # }
/// ```
#[macro_export]
macro_rules! assert_parse {
    ($source:expr) => {{
        let context = runestick::Context::with_default_modules().unwrap();
        $crate::compile_source(&context, $source).unwrap()
    }};
}

/// Assert that the given rune program raises a compile error.
///
/// # Examples
///
/// ```rust
/// use rune_testing::*;
///
/// # fn main() {
/// assert_compile_error! {
///     r#"fn main() { break; }"#,
///     BreakOutsideOfLoop { span } => {
///         assert_eq!(span, Span::new(12, 17));
///     }
/// };
/// # }
/// ```
#[macro_export]
macro_rules! assert_compile_error {
    ($source:expr, $pat:pat => $cond:expr) => {{
        let context = runestick::Context::with_default_modules().unwrap();
        let e = $crate::compile_source(&context, $source).unwrap_err();
        let e = e.into_iter().next().expect("expected one error");

        match e.into_kind() {
            rune::LoadErrorKind::CompileError($pat) => ($cond),
            kind => {
                panic!("expected error `{}` but was `{:?}`", stringify!($pat), kind);
            }
        }
    }};
}

/// Assert that the given rune program parses, but raises the specified set of
/// warnings.
///
/// # Examples
///
/// ```rust
/// use rune_testing::*;
///
/// # fn main() {
/// assert_warnings! {
///     r#"fn main() { `Hello World` }"#,
///     TemplateWithoutExpansions { span, .. } => {
///         assert_eq!(span, Span::new(12, 25));
///     }
/// };
/// # }
/// ```
#[macro_export]
macro_rules! assert_warnings {
    ($source:expr $(, $pat:pat => $cond:expr)*) => {{
        let context = runestick::Context::with_default_modules().unwrap();
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
