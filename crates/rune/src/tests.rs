//! Test cases for rune.

#![allow(clippy::bool_assert_comparison)]
#![allow(clippy::approx_constant)]
#![cfg_attr(miri, allow(unused))]

/// A convenience prelude for tests.
///
/// Import by adding `prelude!();` to your test module.
#[allow(unused_imports)]
pub(crate) mod prelude {
    pub(crate) use crate as rune;
    pub(crate) use crate::alloc;
    pub(crate) use crate::alloc::fmt::TryWrite;
    pub(crate) use crate::alloc::prelude::*;
    pub(crate) use crate::ast;
    pub(crate) use crate::compile::{self, ErrorKind, Located, Named};
    pub(crate) use crate::diagnostics::{self, WarningDiagnosticKind};
    pub(crate) use crate::hash;
    pub(crate) use crate::macros;
    pub(crate) use crate::module::InstallWith;
    pub(crate) use crate::parse;
    pub(crate) use crate::runtime::{
        self, Address, Bytes, DynamicTuple, Formatter, Function, MaybeTypeOf, Object, Output,
        OwnedTuple, Protocol, RawAnyGuard, Ref, Shared, Stack, Tuple, TypeHash, TypeInfo, TypeOf,
        UnsafeToRef, VecTuple, VmErrorKind,
    };
    pub(crate) use crate::support::Result;
    pub(crate) use crate::sync::Arc;
    pub(crate) use crate::tests::{eval, run};
    pub(crate) use crate::{
        from_value, prepare, sources, span, Any, Context, ContextError, Diagnostics, FromValue,
        Hash, Item, ItemBuf, Module, Options, Source, Sources, Value, Vm,
    };
    pub(crate) use futures_executor::block_on;

    pub(crate) use rust_alloc::string::{String, ToString};
    pub(crate) use rust_alloc::vec::Vec;

    pub(crate) use anyhow::Context as AnyhowContext;
}

use core::fmt;

use rust_alloc::string::String;

use anyhow::{Context as _, Error, Result};

use crate::runtime::{Args, VmError};
use crate::sync::Arc;
use crate::{
    alloc, termcolor, BuildError, Context, Diagnostics, FromValue, Hash, Options, Source, Sources,
    Unit, Vm,
};

/// An error that can be raised during testing.
#[derive(Debug)]
pub enum TestError {
    /// A load error was raised during testing.
    Error(Error),
    /// A virtual machine error was raised during testing.
    VmError(VmError),
}

impl From<Error> for TestError {
    fn from(error: Error) -> Self {
        TestError::Error(error)
    }
}

impl From<alloc::Error> for TestError {
    fn from(error: alloc::Error) -> Self {
        TestError::Error(Error::new(error))
    }
}

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TestError::Error(error) => write!(f, "Build error: {error}"),
            TestError::VmError(error) => write!(f, "Vm error: {error}"),
        }
    }
}

impl core::error::Error for TestError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            TestError::Error(error) => Some(error.as_ref()),
            TestError::VmError(error) => Some(error),
        }
    }
}

/// Compile the given source into a unit and collection of warnings.
#[doc(hidden)]
pub fn compile_helper(source: &str, diagnostics: &mut Diagnostics) -> Result<Unit, BuildError> {
    let context = crate::Context::with_default_modules().expect("setting up default modules");

    let mut sources = Sources::new();
    sources.insert(Source::new("main", source)?)?;

    let mut options = Options::default();
    options.script(true);

    let unit = crate::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(diagnostics)
        .with_options(&options)
        .build()?;

    Ok(unit)
}

/// Construct a virtual machine for the given sources.
#[doc(hidden)]
pub fn vm(
    context: &Context,
    sources: &mut Sources,
    diagnostics: &mut Diagnostics,
    script: bool,
) -> Result<Vm, TestError> {
    let runtime = Arc::try_new(context.runtime()?)?;

    let mut options = Options::default();

    if script {
        options.script(true);
    }

    let result = crate::prepare(sources)
        .with_context(context)
        .with_diagnostics(diagnostics)
        .with_options(&options)
        .build();

    let Ok(unit) = result else {
        let mut buffer = termcolor::Buffer::no_color();

        diagnostics
            .emit(&mut buffer, sources)
            .context("Emit diagnostics")?;

        let error = Error::msg(String::from_utf8(buffer.into_inner()).context("Non utf-8 output")?);
        return Err(TestError::Error(error));
    };

    let unit = Arc::try_new(unit)?;
    Ok(Vm::new(runtime, unit))
}

/// Call the specified function in the given script sources.
#[doc(hidden)]
pub fn run_helper<T>(
    context: &Context,
    sources: &mut Sources,
    diagnostics: &mut Diagnostics,
    args: impl Args,
    script: bool,
) -> Result<T, TestError>
where
    T: FromValue,
{
    let mut vm = vm(context, sources, diagnostics, script)?;

    let mut execute = if script {
        vm.execute(Hash::EMPTY, args).map_err(TestError::VmError)?
    } else {
        vm.execute(["main"], args).map_err(TestError::VmError)?
    };

    let output = ::futures_executor::block_on(execute.resume())
        .map_err(TestError::VmError)?
        .into_complete()
        .map_err(TestError::VmError)?;

    crate::from_value(output).map_err(|error| TestError::VmError(error.into()))
}

#[doc(hidden)]
pub fn sources(source: &str) -> Sources {
    let mut sources = Sources::new();
    sources
        .insert(Source::new("main", source).expect("Failed to build source"))
        .expect("Failed to insert source");
    sources
}

/// Run the given source with diagnostics being printed to stderr.
pub fn run<T>(context: &Context, source: &str, args: impl Args, script: bool) -> Result<T>
where
    T: FromValue,
{
    let mut sources = Sources::new();
    sources.insert(Source::memory(source)?)?;

    let mut diagnostics = Default::default();

    let e = match run_helper(context, &mut sources, &mut diagnostics, args, script) {
        Ok(value) => return Ok(value),
        Err(e) => e,
    };

    match e {
        TestError::Error(error) => Err(error),
        TestError::VmError(e) => {
            let mut buffer = termcolor::Buffer::no_color();
            e.emit(&mut buffer, &sources).context("Emit diagnostics")?;
            let buffer =
                String::from_utf8(buffer.into_inner()).context("Decode output as utf-8")?;
            Err(Error::msg(buffer))
        }
    }
}

/// Generate an expectation panic.
macro_rules! expected {
    ($name:literal, $expected:pat, $actual:expr) => {
        panic!(
            "Did not match expected {}\nExpected: {}\n  Actual: {:?}",
            $name,
            stringify!($expected),
            $actual,
        )
    };

    ($name:literal, $expected:pat, $actual:expr, $extra:expr) => {
        panic!(
            "Did not match expected {}\nExpected: {}\n  Actual: {:?}\n{}",
            $name,
            stringify!($expected),
            $actual,
            $extra,
        )
    };
}

#[track_caller]
pub(crate) fn eval<T>(source: impl AsRef<str>) -> T
where
    T: FromValue,
{
    let source = source.as_ref();
    let context = Context::with_default_modules().expect("Failed to build context");

    match run(&context, source, (), true) {
        Ok(output) => output,
        Err(error) => {
            panic!("Program failed to run:\n{error}\n{source}");
        }
    }
}

/// Evaluate a Rust token tree. This works fairly well because Rust and Rune has
/// very similar token trees.
macro_rules! rune {
    ($($tt:tt)*) => {
        $crate::tests::eval(stringify!($($tt)*))
    };
}

/// Assert that the given source evaluates to `true`.
macro_rules! rune_assert {
    ($($tt:tt)*) => {{
        let value: bool = $crate::tests::eval(stringify!($($tt)*));
        assert!(value, "Rune program is not `true`:\n{}", stringify!($($tt)*));
    }};
}

/// Same as [rune!] macro, except it takes an external context, allowing testing
/// of native Rust data. This also accepts a tuple of arguments in the second
/// position, to pass native objects as arguments to the script.
macro_rules! rune_n {
    ($(mod $module:expr,)* $args:expr, $($tt:tt)*) => {{
        let mut context = $crate::Context::with_default_modules().expect("Failed to build context");
        $(context.install(&$module).expect("Failed to install native module");)*
        $crate::tests::run(&context, stringify!($($tt)*), $args, false).expect("Program ran unsuccessfully")
    }};
}

/// Assert that the given vm error happens with the given rune program.
macro_rules! assert_vm_error {
    // Second variant which allows for specifyinga type.
    ($source:expr, $pat:pat => $cond:block) => {
        assert_vm_error!(() => $source, $pat => $cond)
    };

    // Second variant which allows for specifyinga type.
    ($ty:ty => $source:expr, $pat:pat => $cond:block) => {{
        let context = $crate::Context::with_default_modules().unwrap();
        let mut diagnostics = Default::default();

        let mut sources = $crate::tests::sources($source);
        let e = match $crate::tests::run_helper::<$ty>(&context, &mut sources, &mut diagnostics, (), true) {
            Err(e) => e,
            actual => {
                expected!("program error", Err(e), actual, $source)
            }
        };

        let e = match e {
            $crate::tests::TestError::VmError(e) => e,
            actual => {
                expected!("vm error", VmError(e), actual, $source)
            }
        };

        match e.into_kind() {
            $pat => $cond,
            actual => {
                expected!("error", $pat, actual, $source)
            }
        }
    }};
}

/// Assert that the given rune program parses.
macro_rules! assert_parse {
    ($source:expr) => {{
        let mut diagnostics = Default::default();
        $crate::tests::compile_helper($source, &mut diagnostics).unwrap()
    }};
}

/// Assert that the given rune program raises a query error.
macro_rules! assert_errors {
    ($source:expr, $span:pat $(, $pat:pat $(=> $cond:expr)?)+ $(,)?) => {{
        let mut diagnostics = Default::default();
        let _ = $crate::tests::compile_helper($source, &mut diagnostics).unwrap_err();

        let mut it = diagnostics.into_diagnostics().into_iter();

        $(
            let e = match it.next().expect("expected error") {
                rune::diagnostics::Diagnostic::Fatal(e) => e,
                actual => {
                    expected!("fatal diagnostic", Fatal(e), actual)
                }
            };

            let e = match e.into_kind() {
                rune::diagnostics::FatalDiagnosticKind::CompileError(e) => (e),
                actual => {
                    expected!("compile error", CompileError(e), actual)
                }
            };

            let span = rune::ast::Spanned::span(&e);

            #[allow(irrefutable_let_patterns)]
            let $span = span else {
                expected!("span", $span, span)
            };

            match e.into_kind() {
                $pat => {$($cond)*},
                #[allow(unreachable_patterns)]
                actual => {
                    expected!("error", $pat, actual)
                }
            }
        )+
    }};
}

/// Assert that the given rune program parses, but raises the specified set of
/// warnings.
macro_rules! assert_warnings {
    ($source:expr, $span:pat $(, $pat:pat $(=> $cond:expr)?)* $(,)?) => {{
        let mut diagnostics = Default::default();
        let _ = $crate::tests::compile_helper($source, &mut diagnostics).expect("source should compile");
        assert!(diagnostics.has_warning(), "no warnings produced");

        let mut it = diagnostics.into_diagnostics().into_iter();

        $(
            let warning = it.next().expect("expected a warning");

            let warning = match warning {
                rune::diagnostics::Diagnostic::Warning(warning) => warning,
                actual => {
                    expected!("warning diagnostic", $pat, actual)
                }
            };

            let span = rune::ast::Spanned::span(&warning);

            #[allow(irrefutable_let_patterns)]
            let $span = span else {
                expected!("span", $span, span)
            };

            match warning.into_kind() {
                $pat => {$($cond)*},
                actual => {
                    expected!("warning", $pat, actual)
                }
            }
        )*

        assert!(it.next().is_none(), "there should be no more warnings");
    }};
}

/// Assert that the given value matches the provided pattern.
macro_rules! assert_matches {
    ($value:expr, $pat:pat) => {
        match $value {
            $pat => (),
            other => panic!("expected {}, but was {:?}", stringify!($pat), other),
        }
    };
}

macro_rules! prelude {
    () => {
        #[allow(unused_imports)]
        use crate::tests::prelude::*;
    };
}

#[cfg(not(miri))]
mod attribute;
#[cfg(not(miri))]
mod binary;
#[cfg(not(miri))]
mod bug_326;
#[cfg(not(miri))]
mod bug_344;
#[cfg(not(miri))]
mod bug_417;
#[cfg(not(miri))]
mod bug_422;
#[cfg(not(miri))]
mod bug_428;
#[cfg(not(miri))]
mod bug_454;
#[cfg(not(miri))]
mod bug_700;
#[cfg(not(miri))]
mod bug_905;
#[cfg(not(miri))]
mod bugfixes;
#[cfg(not(miri))]
mod builtin_macros;
#[cfg(not(miri))]
mod capture;
#[cfg(not(miri))]
mod comments;
#[cfg(not(miri))]
mod compiler_docs;
#[cfg(not(miri))]
mod compiler_expr_assign;
#[cfg(not(miri))]
mod compiler_fn;
#[cfg(not(miri))]
mod compiler_general;
#[cfg(not(miri))]
mod compiler_paths;
#[cfg(not(miri))]
mod compiler_patterns;
#[cfg(not(miri))]
mod compiler_use;
#[cfg(not(miri))]
mod compiler_visibility;
#[cfg(not(miri))]
mod compiler_warnings;
#[cfg(not(miri))]
mod continue_;
#[cfg(not(miri))]
mod core_macros;
#[cfg(not(miri))]
mod custom_macros;
#[cfg(not(miri))]
mod debug_fmt;
#[cfg(not(miri))]
mod deprecation;
#[cfg(not(miri))]
mod derive_constructor;
#[cfg(not(miri))]
mod destructuring;
#[cfg(not(miri))]
mod esoteric_impls;
#[cfg(not(miri))]
mod external_constructor;
#[cfg(not(miri))]
mod external_generic;
#[cfg(not(miri))]
mod external_match;
#[cfg(not(miri))]
mod external_ops;
#[cfg(not(miri))]
mod f64;
mod function_guardedargs;
#[cfg(not(miri))]
mod getter_setter;
#[cfg(not(miri))]
mod iterator;
#[cfg(not(miri))]
mod literals;
#[cfg(not(miri))]
mod macros;
#[cfg(not(miri))]
mod moved;
#[cfg(not(miri))]
mod option;
#[cfg(not(miri))]
mod patterns;
#[cfg(not(miri))]
mod quote;
#[cfg(not(miri))]
mod range;
#[cfg(not(miri))]
mod reference_error;
#[cfg(not(miri))]
mod rename_type;
#[cfg(not(miri))]
mod result;
#[cfg(not(miri))]
mod static_typing;
#[cfg(not(miri))]
mod tuple;
#[cfg(not(miri))]
mod type_name_native;
#[cfg(not(miri))]
mod unit_constants;
#[cfg(not(miri))]
mod unreachable;
#[cfg(not(miri))]
mod vm_arithmetic;
#[cfg(not(miri))]
mod vm_assign_exprs;
#[cfg(not(miri))]
mod vm_async_block;
#[cfg(not(miri))]
mod vm_blocks;
#[cfg(not(miri))]
mod vm_closures;
#[cfg(not(miri))]
mod vm_const_exprs;
#[cfg(not(miri))]
mod vm_early_termination;
#[cfg(not(miri))]
mod vm_function;
#[cfg(not(miri))]
mod vm_function_pointers;
#[cfg(not(miri))]
mod vm_general;
#[cfg(not(miri))]
mod vm_not_used;
#[cfg(not(miri))]
mod vm_result;
#[cfg(not(miri))]
mod vm_test_from_value_derive;
#[cfg(not(miri))]
mod vm_test_imports;
#[cfg(not(miri))]
mod vm_test_instance_fns;
#[cfg(not(miri))]
mod vm_test_linked_list;
#[cfg(not(miri))]
mod vm_test_mod;
#[cfg(not(miri))]
mod vm_try;
#[cfg(not(miri))]
mod wildcard_imports;
#[cfg(not(miri))]
mod workspace;
