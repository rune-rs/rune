//! Utilities used for testing small Rune programs.
//!
//! This module can be disabled through the `testing` feature.

pub use crate::CompileErrorKind::*;
pub use crate::ParseErrorKind::*;
pub use crate::WarningKind::*;
use crate::{Error, Errors, Sources, UnitBuilder, Warnings};
pub use futures_executor::block_on;
pub use runestick::VmErrorKind::*;
pub use runestick::{
    CompileMeta, CompileMetaKind, ContextError, FromValue, Function, IntoComponent, Span, ToValue,
    Value, VecTuple, VmError,
};
use runestick::{Item, Source, Unit};
use std::sync::Arc;
use thiserror::Error;

/// An error that can be raised during testing.
#[derive(Debug, Error)]
pub enum RunError {
    /// A load error was raised during testing.
    #[error("load errors")]
    Errors(Errors),
    /// A virtual machine error was raised during testing.
    #[error("vm error")]
    VmError(#[source] VmError),
    /// A context error was raised during testing.
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
    let unit = UnitBuilder::with_default_prelude();

    if let Err(()) = crate::compile(context, &mut sources, &unit, &mut errors, &mut warnings) {
        return Err(errors);
    }

    let unit = match unit.build() {
        Some(unit) => unit,
        None => {
            errors.push(Error::internal(0, "unit builder is not exclusively held"));
            return Err(errors);
        }
    };

    Ok((unit, warnings))
}

/// Call the specified function in the given script.
pub async fn run_async<N, A, T>(function: N, args: A, source: &str) -> Result<T, RunError>
where
    N: IntoIterator,
    N::Item: IntoComponent,
    A: runestick::Args,
    T: FromValue,
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

/// Helper function to construct a context and unit from a Rune source for
/// testing purposes.
///
/// This is primarily used in examples.
pub fn build(source: &str) -> runestick::Result<(Arc<runestick::Context>, Arc<runestick::Unit>)> {
    let context = std::sync::Arc::new(runestick::Context::with_default_modules()?);
    let options = crate::Options::default();
    let mut sources = crate::Sources::new();
    sources.insert(runestick::Source::new("source", source));

    let mut warnings = crate::Warnings::new();
    let mut errors = crate::Errors::new();

    let unit = match crate::load_sources(
        &*context,
        &options,
        &mut sources,
        &mut errors,
        &mut warnings,
    ) {
        Ok(unit) => unit,
        Err(error) => {
            let mut writer =
                crate::termcolor::StandardStream::stderr(crate::termcolor::ColorChoice::Always);
            crate::EmitDiagnostics::emit_diagnostics(&errors, &mut writer, &sources)?;
            return Err(error.into());
        }
    };

    if !warnings.is_empty() {
        let mut writer =
            crate::termcolor::StandardStream::stderr(crate::termcolor::ColorChoice::Always);
        crate::EmitDiagnostics::emit_diagnostics(&warnings, &mut writer, &sources)?;
    }

    Ok((context, std::sync::Arc::new(unit)))
}

/// Run the given program and return the expected type from it.
///
/// # Examples
///
/// ```rust
/// use rune::testing::*;
///
/// # fn main() {
/// assert_eq! {
///     rune::rune!(bool => r#"fn main() { true || false }"#),
///     true,
/// };
/// # }
/// ```
#[macro_export]
macro_rules! rune {
    ($ty:ty => $source:expr) => {
        $crate::testing::run::<_, (), $ty>(&["main"], (), $source)
            .expect("program to run successfully")
    };
}

/// Assert that the given parse error happens with the given rune program.
///
/// # Examples
///
/// ```rust
/// use rune::testing::*;
///
/// # fn main() {
/// rune::assert_parse_error! {
///     r#"fn main() { 0 < 10 >= 10 }"#,
///     span, PrecedenceGroupRequired => {
///         assert_eq!(span, Span::new(12, 18));
///     }
/// };
/// # }
/// ```
#[macro_export]
macro_rules! assert_parse_error {
    ($source:expr, $span:ident, $pat:pat => $cond:expr) => {{
        let context = runestick::Context::with_default_modules().unwrap();
        let errors = $crate::testing::compile_source(&context, &$source).unwrap_err();
        let err = errors.into_iter().next().expect("expected one error");

        let e = match err.into_kind() {
            $crate::ErrorKind::ParseError(e) => (e),
            kind => {
                panic!(
                    "expected parse error `{}` but was `{:?}`",
                    stringify!($pat),
                    kind
                );
            }
        };

        let $span = $crate::Spanned::span(&e);

        match e.into_kind() {
            $pat => $cond,
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
/// use rune::testing::*;
///
/// # fn main() {
/// rune::assert_vm_error!(
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
        let e = $crate::testing::run::<_, _, $ty>(&["main"], (), $source).unwrap_err();

        let (e, _) = match e {
            $crate::testing::RunError::VmError(e) => e.into_unwound(),
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
/// # fn main() {
/// rune::assert_parse!(r#"fn main() { (0 < 10) >= 10 }"#);
/// # }
/// ```
#[macro_export]
macro_rules! assert_parse {
    ($source:expr) => {{
        let context = runestick::Context::with_default_modules().unwrap();
        $crate::testing::compile_source(&context, $source).unwrap()
    }};
}

/// Assert that the given rune program raises a compile error.
///
/// # Examples
///
/// ```rust
/// use rune::testing::*;
///
/// # fn main() {
/// rune::assert_compile_error! {
///     r#"fn main() { break; }"#,
///     span, BreakOutsideOfLoop => {
///         assert_eq!(span, Span::new(12, 17));
///     }
/// };
/// # }
/// ```
#[macro_export]
macro_rules! assert_compile_error {
    ($source:expr, $span:ident, $pat:pat => $cond:expr) => {{
        let context = runestick::Context::with_default_modules().unwrap();
        let e = $crate::testing::compile_source(&context, $source).unwrap_err();
        let e = e.into_iter().next().expect("expected one error");

        let e = match e.into_kind() {
            $crate::ErrorKind::CompileError(e) => (e),
            kind => {
                panic!(
                    "expected parse error `{}` but was `{:?}`",
                    stringify!($pat),
                    kind
                );
            }
        };

        let $span = $crate::Spanned::span(&e);

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
///
/// # Examples
///
/// ```rust
/// use rune::testing::*;
///
/// # fn main() {
/// rune::assert_warnings! {
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
        let (_, warnings) = $crate::testing::compile_source(&context, $source).expect("source should compile");
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

/// Function used during parse testing to take the source, parse it as the given
/// type, tokenize it using [ToTokens][crate::macros::ToTokens], and parse the
/// token stream.
///
/// The results should be identical.
pub fn roundtrip<T>(source: &str) -> T
where
    T: crate::parsing::Parse + crate::macros::ToTokens + PartialEq + Eq + std::fmt::Debug,
{
    let mut parser = crate::parsing::Parser::new(source);
    let ast = parser.parse::<T>().expect("first parse");
    parser.parse_eof().expect("first parse eof");

    let mut ctx = crate::macros::MacroContext::empty();
    let mut token_stream = crate::macros::TokenStream::empty();

    ast.to_tokens(&mut ctx, &mut token_stream);
    let mut parser = crate::parsing::Parser::from_token_stream(&token_stream);
    let ast2 = parser.parse::<T>().expect("second parse");
    parser.parse_eof().expect("second parse eof");

    assert_eq!(ast, ast2);
    ast
}
