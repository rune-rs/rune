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

/// Run the given program as a test.
#[macro_export]
macro_rules! test {
    ($ty:ty => $source:expr) => {
        block_on($crate::run_main::<$ty>($source)).expect("program to run successfully")
    };
}

#[macro_export]
macro_rules! test_vm_error {
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

#[macro_export]
macro_rules! test_parse {
    ($source:expr) => {{
        let context = runestick::Context::with_default_packages().unwrap();
        rune::compile(&context, $source).unwrap();
    }};
}

#[macro_export]
macro_rules! test_compile_error {
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

#[macro_export]
macro_rules! test_warnings {
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

#[macro_export]
macro_rules! test_parse_error {
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
