pub use futures_executor::block_on;
pub use rune::CompileError::*;
pub use rune::ParseError::*;
pub use rune::Warning::*;
use runestick::Item;
pub use runestick::VmError::*;
pub use runestick::{FnPtr, Meta, Span, Value};
use std::rc::Rc;

pub async fn run_main<T>(source: &str) -> Result<T, Box<dyn std::error::Error>>
where
    T: runestick::FromValue,
{
    let context = runestick::Context::with_default_packages()?;
    let (unit, _) = rune::compile(&context, source)?;
    let mut vm = runestick::Vm::new(Rc::new(context), Rc::new(unit));
    let mut task: runestick::Task<T> = vm.call_function(Item::of(&["main"]), ())?;
    let output = task.run_to_completion().await?;
    Ok(output)
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
        block_on($crate::run_main::<$ty>($source)).expect("program to run successfully")
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
        let context = runestick::Context::with_default_packages().unwrap();
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
        let e = block_on($crate::run_main::<()>($source)).unwrap_err();

        let e = match e.downcast_ref::<runestick::VmError>() {
            Some(e) => e,
            None => {
                panic!("{:?}", e);
            }
        };

        match e {
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
        let context = runestick::Context::with_default_packages().unwrap();
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
        let context = runestick::Context::with_default_packages().unwrap();
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
        let context = runestick::Context::with_default_packages().unwrap();
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
