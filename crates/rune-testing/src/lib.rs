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
//! A crate use to simplify testing small rune programs.
//!
//! This is a crate used with the [Rune language].
//!
//! [Rune Language]: https://github.com/rune-rs/rune

pub use futures_executor::block_on;
pub use rune::CompileError::*;
pub use rune::ParseError::*;
pub use rune::WarningKind::*;
use rune::Warnings;
pub use runestick::VmErrorKind::*;
use runestick::{Component, Item, Source, Unit};
pub use runestick::{Function, Meta, Span, Value};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

/// The result returned from our functions.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The error returned from our functions.
pub type Error = Box<dyn std::error::Error + 'static + Send + Sync>;

/// Compile the given source into a unit and collection of warnings.
pub fn compile_source(
    context: &runestick::Context,
    source: &str,
) -> Result<(Unit, Warnings), rune::CompileError> {
    let source = Source::new("main", source.to_owned());
    let unit = Rc::new(RefCell::new(Unit::with_default_prelude()));
    let mut warnings = Warnings::new();

    rune::compile(context, &source, &unit, &mut warnings)?;

    let unit = Rc::try_unwrap(unit).unwrap().into_inner();
    Ok((unit, warnings))
}

/// Call the specified function in the given script.
pub async fn run_async<N, A, T>(function: N, args: A, source: &str) -> Result<T>
where
    N: IntoIterator,
    N::Item: Into<Component>,
    A: runestick::Args,
    T: runestick::FromValue,
{
    let context = runestick::Context::with_default_modules()?;
    let (unit, _) = compile_source(&context, &source)?;

    let vm = runestick::Vm::new(Arc::new(context), Arc::new(unit));
    let output = vm.call(Item::of(function), args)?.async_complete().await?;

    Ok(T::from_value(output)?)
}

/// Call the specified function in the given script.
pub fn run<N, A, T>(function: N, args: A, source: &str) -> Result<T>
where
    N: IntoIterator,
    N::Item: Into<Component>,
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
        let err = $crate::compile_source(&context, &$source).unwrap_err();

        match err {
            rune::CompileError::ParseError { error: $pat } => ($cond),
            _ => {
                panic!("expected error `{}` but was `{:?}`", stringify!($pat), err);
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
    ($source:expr, $pat:pat => $cond:expr) => {{
        let e = $crate::run::<_, _, ()>(&["main"], (), $source).unwrap_err();

        let e = match e.downcast_ref::<runestick::VmError>() {
            Some(e) => e,
            None => {
                panic!("{:?}", e);
            }
        };

        let (kind, _) = e.kind().into_unwound_ref();

        match kind {
            $pat => $cond,
            _ => {
                panic!("expected error `{}` but was `{:?}`", stringify!($pat), e);
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
        let err = $crate::compile_source(&context, $source).unwrap_err();

        match err {
            $pat => ($cond),
            _ => {
                panic!("expected error `{}` but was `{:?}`", stringify!($pat), err);
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
