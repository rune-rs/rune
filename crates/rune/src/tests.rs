//! Test cases for rune.

#![allow(clippy::bool_assert_comparison)]
#![allow(clippy::approx_constant)]

pub(crate) mod prelude {
    pub(crate) use crate as rune;
    pub(crate) use crate::ast;
    pub(crate) use crate::compile::{
        self, CompileErrorKind, Item, Located, Named, ParseErrorKind, QueryErrorKind,
        ResolveErrorKind,
    };
    pub(crate) use crate::diagnostics;
    pub(crate) use crate::macros;
    pub(crate) use crate::module::InstallWith;
    pub(crate) use crate::parse;
    pub(crate) use crate::runtime::{
        self, AnyObj, AnyTypeInfo, Bytes, FullTypeOf, Function, MaybeTypeOf, Object, Protocol,
        RawRef, RawStr, Shared, Stack, Tuple, TypeInfo, TypeOf, UnsafeFromValue, VecTuple,
        VmErrorKind, VmResult,
    };
    pub(crate) use crate::tests::run;
    pub(crate) use crate::{
        from_value, prepare, sources, span, vm_try, Any, Context, ContextError, Diagnostics,
        FromValue, Hash, Module, Result, Source, Sources, Value, Vm,
    };
    pub(crate) use futures_executor::block_on;
}

use crate::no_std::prelude::*;
use crate::no_std::sync::Arc;

use anyhow::{Context as _, Error, Result};
use thiserror::Error;

use crate::compile::{IntoComponent, ItemBuf};
use crate::runtime::{Args, VmError, VmResult};
use crate::{termcolor, BuildError, Context, Diagnostics, FromValue, Source, Sources, Unit, Vm};

/// An error that can be raised during testing.
#[derive(Debug, Error)]
pub enum RunError {
    /// A load error was raised during testing.
    #[error("Build error: {0}")]
    BuildError(String),
    /// A virtual machine error was raised during testing.
    #[error("Vm error: {0}")]
    VmError(VmError),
}

/// Compile the given source into a unit and collection of warnings.
#[doc(hidden)]
pub fn compile_helper(source: &str, diagnostics: &mut Diagnostics) -> Result<Unit, BuildError> {
    let context = crate::Context::with_default_modules().expect("setting up default modules");

    let mut sources = Sources::new();
    sources.insert(Source::new("main", source));

    let unit = crate::prepare(&mut sources)
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
    let result = crate::prepare(sources)
        .with_context(context)
        .with_diagnostics(diagnostics)
        .build();

    let Ok(unit) = result else {
        let mut buffer = termcolor::Buffer::no_color();

        diagnostics
            .emit(&mut buffer, sources)
            .expect("Emit diagnostics");

        let buffer = String::from_utf8(buffer.into_inner()).expect("Non utf-8 output");
        return Err(RunError::BuildError(buffer))
    };

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
pub fn run<N, A, T>(context: &Context, source: &str, function: N, args: A) -> Result<T>
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

    match e {
        RunError::BuildError(string) => Err(Error::msg(string)),
        RunError::VmError(e) => {
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
            "Did not get expected {}\nExpected: {}\n  Actual: {:?}",
            $name,
            stringify!($expected),
            $actual
        )
    };
}

/// Same as [rune_s!] macro, except it takes a Rust token tree. This works
/// fairly well because Rust and Rune has very similar token trees.
macro_rules! rune {
    ($($tt:tt)*) => {{
        let context = $crate::Context::with_default_modules().expect("Failed to build context");
        $crate::tests::run(&context, stringify!($($tt)*), ["main"], ()).expect("Program ran unsuccessfully")
    }};
}

/// Run the given program and return the expected type from it.
macro_rules! rune_s {
    ($source:expr) => {{
        let context = $crate::Context::with_default_modules().expect("Failed to build context");
        $crate::tests::run(&context, $source, ["main"], ()).expect("Program ran unsuccessfully")
    }};
}

/// Same as [rune!] macro, except it takes an external context, allowing testing
/// of native Rust data. This also accepts a tuple of arguments in the second
/// position, to pass native objects as arguments to the script.
macro_rules! rune_n {
    ($module:expr, $args:expr, $ty:ty => $($tt:tt)*) => {{
        let mut context = $crate::Context::with_default_modules().expect("Failed to build context");
        context.install($module).expect("failed to install native module");
        $crate::tests::run::<_, _, $ty>(&context, stringify!($($tt)*), ["main"], $args).expect("Program ran unsuccessfully")
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
        let e = match $crate::tests::run_helper::<_, _, $ty>(&context, &mut sources, &mut diagnostics, ["main"], ()) {
            Err(e) => e,
            actual => {
                expected!("program error", Err(e), actual)
            }
        };

        let e = match e {
            $crate::tests::RunError::VmError(e) => e,
            actual => {
                expected!("vm error", VmError(e), actual)
            }
        };

        match e.into_kind() {
            $pat => $cond,
            actual => {
                expected!("error", $pat, actual)
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
    ($source:expr, $span:pat, $($pat:pat $(=> $cond:expr)?),+ $(,)?) => {{
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
    ($source:expr, $span:pat $(, $pat:pat $(=> $cond:expr)?)*) => {{
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
        use crate::no_std::prelude::*;
        #[allow(unused_imports)]
        use crate::tests::prelude::*;
    };
}

mod attribute;
mod binary;
mod bug_326;
mod bug_344;
mod bug_417;
mod bug_422;
mod bug_428;
mod bug_454;
mod bugfixes;
mod capture;
mod char;
mod collections;
mod comments;
mod compiler_docs;
mod compiler_expr_assign;
mod compiler_fn;
mod compiler_general;
mod compiler_literals;
mod compiler_paths;
mod compiler_patterns;
mod compiler_use;
mod compiler_visibility;
mod compiler_warnings;
mod continue_;
mod core_macros;
mod custom_macros;
mod destructuring;
mod external_ops;
mod float;
mod for_loop;
mod generics;
mod getter_setter;
mod instance;
mod int;
mod iter;
mod iterator;
mod macros;
mod match_external;
mod moved;
mod option;
mod patterns;
mod quote;
mod range;
mod reference_error;
mod result;
mod stmt_reordering;
mod type_name_native;
mod type_name_rune;
mod unit_constants;
mod variants;
mod vm_arithmetic;
mod vm_assign_exprs;
mod vm_async_block;
mod vm_blocks;
mod vm_closures;
mod vm_const_exprs;
mod vm_early_termination;
mod vm_function;
mod vm_general;
mod vm_generators;
mod vm_is;
mod vm_lazy_and_or;
mod vm_literals;
mod vm_match;
mod vm_not_used;
mod vm_option;
mod vm_pat;
mod vm_result;
mod vm_streams;
mod vm_test_external_fn_ptr;
mod vm_test_from_value_derive;
mod vm_test_imports;
mod vm_test_instance_fns;
mod vm_test_linked_list;
mod vm_test_mod;
mod vm_try;
mod vm_tuples;
mod vm_typed_tuple;
mod vm_types;
mod wildcard_imports;
