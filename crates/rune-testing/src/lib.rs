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
pub use rune::Warning::*;
pub use runestick::VmErrorKind::*;
use runestick::{Component, Item};
pub use runestick::{FnPtr, Meta, Span, Value};
use std::rc::Rc;

/// The result returned from our functions.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The error returned from our functions.
pub type Error = Box<dyn std::error::Error + 'static + Send + Sync>;

/// Call the specified function in the given script.
pub async fn run_async<N, A, T>(function: N, args: A, source: &str) -> Result<T>
where
    N: IntoIterator,
    N::Item: Into<Component>,
    A: runestick::IntoArgs,
    T: runestick::FromValue,
{
    let context = runestick::Context::with_default_modules()?;
    let (unit, _) = rune::compile(&context, source)?;
    let mut vm = runestick::Vm::new(Rc::new(context), Rc::new(unit));
    let mut task: runestick::Task<T> = vm.call_function(Item::of(function), args)?;
    let output = task.run_to_completion().await?;
    Ok(output)
}

/// Call the specified function in the given script.
pub fn run<N, A, T>(function: N, args: A, source: &str) -> Result<T>
where
    N: IntoIterator,
    N::Item: Into<Component>,
    A: runestick::IntoArgs,
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
        let err = rune::compile(&context, $source).unwrap_err();

        match err {
            rune::Error::ParseError($pat) => ($cond),
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

        let (kind, _) = e.kind().from_unwinded_ref();

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
        rune::compile(&context, $source).unwrap();
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
        let err = rune::compile(&context, $source).unwrap_err();

        match err {
            rune::Error::CompileError($pat) => ($cond),
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
///     r#"fn main() { loop { let _ = break; } }"#,
///     BreakDoesNotProduceValue { span, .. } => {
///         assert_eq!(span, Span::new(27, 32));
///     }
/// };
/// # }
/// ```
#[macro_export]
macro_rules! assert_warnings {
    ($source:expr $(, $pat:pat => $cond:expr)*) => {{
        let context = runestick::Context::with_default_modules().unwrap();
        let (_, warnings) = rune::compile(&context, $source).expect("source should compile");
        assert!(!warnings.is_empty(), "no warnings produced");

        let mut it = warnings.into_iter();

        $(
            let warning = it.next().expect("expected a warning");

            match warning {
                $pat => ($cond),
                warning => {
                    panic!("expected warning `{}` but was `{:?}`", stringify!($pat), warning);
                }
            }
        )*

        assert!(it.next().is_none(), "there should be no more warnings");
    }};
}
