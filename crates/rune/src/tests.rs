//! Test cases for rune.

#![allow(clippy::bool_assert_comparison)]
#![allow(clippy::approx_constant)]

pub(crate) mod prelude {
    pub(crate) use crate as rune;
    pub(crate) use crate::ast;
    pub(crate) use crate::compile::{
        self, CompileErrorKind, Item, Location, Named, ParseErrorKind, ResolveErrorKind,
    };
    pub(crate) use crate::diagnostics;
    pub(crate) use crate::macros;
    pub(crate) use crate::parse;
    pub(crate) use crate::query::QueryErrorKind;
    pub(crate) use crate::runtime::{
        self, AnyObj, AnyTypeInfo, Bytes, Function, MaybeTypeOf, Object, Protocol, RawRef, RawStr,
        Shared, Stack, Tuple, TypeInfo, TypeOf, UnsafeFromValue, VecTuple, VmErrorKind, VmResult,
    };
    pub(crate) use crate::tests::run;
    pub(crate) use crate::{
        from_value, prepare, sources, span, vm_try, Any, Context, ContextError, Diagnostics,
        FromValue, Hash, InstallWith, Module, Result, Source, Sources, ToValue, Value, Vm,
    };
    pub(crate) use futures_executor::block_on;
}

use std::sync::Arc;

use thiserror::Error;

use crate::compile::{IntoComponent, ItemBuf};
use crate::runtime::{Args, VmError, VmResult};
use crate::{termcolor, BuildError, Context, Diagnostics, FromValue, Source, Sources, Unit, Vm};

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
    let unit = crate::prepare(sources)
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

/// Same as [rune_s!] macro, except it takes a Rust token tree. This works
/// fairly well because Rust and Rune has very similar token trees.
macro_rules! rune {
    ($($tt:tt)*) => {{
        let context = $crate::Context::with_default_modules().expect("failed to build context");
        $crate::tests::run(&context, stringify!($($tt)*), ["main"], ()).expect("program to run successfully")
    }};
}

/// Run the given program and return the expected type from it.
macro_rules! rune_s {
    ($source:expr) => {{
        let context = $crate::Context::with_default_modules().expect("failed to build context");
        $crate::tests::run(&context, $source, ["main"], ()).expect("program to run successfully")
    }};
}

/// Same as [rune!] macro, except it takes an external context, allowing testing
/// of native Rust data. This also accepts a tuple of arguments in the second
/// position, to pass native objects as arguments to the script.
macro_rules! rune_n {
    ($module:expr, $args:expr, $ty:ty => $($tt:tt)*) => {{
        let mut context = $crate::Context::with_default_modules().expect("failed to build context");
        context.install($module).expect("failed to install native module");
        $crate::tests::run::<_, _, $ty>(&context, stringify!($($tt)*), ["main"], $args).expect("program to run successfully")
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
            Ok(value) => {
                panic!("expected error but program completed with: {:?}", value);
            }
        };

        let e = match e {
            $crate::tests::RunError::VmError(e) => e,
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
macro_rules! assert_parse {
    ($source:expr) => {{
        let mut diagnostics = Default::default();
        $crate::tests::compile_helper($source, &mut diagnostics).unwrap()
    }};
}

/// Assert that the given rune program raises a query error.
macro_rules! assert_errors {
    ($source:expr, $span:ident, $($variant:ident($pat:pat) => $cond:expr),+ $(,)?) => {{
        let mut diagnostics = Default::default();
        let _ = $crate::tests::compile_helper($source, &mut diagnostics).unwrap_err();

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
                #[allow(unreachable_patterns)]
                kind => {
                    panic!("expected error `{}` but was `{:?}`", stringify!($pat), kind);
                }
            }
        )+
    }};
}

/// Assert that the given rune program raises a compile error.
macro_rules! assert_compile_error {
    ($source:expr, $span:ident, $pat:pat => $cond:expr) => {{
        assert_errors!($source, $span, $pat => $cond)
    }};
}

/// Assert that the given parse error happens with the given rune program.
macro_rules! assert_parse_error {
    ($source:expr, $span:ident, $pat:pat => $cond:expr) => {{
        assert_errors!($source, $span, ParseError($pat) => $cond)
    }};
}

/// Assert that the given rune program parses, but raises the specified set of
/// warnings.
macro_rules! assert_warnings {
    ($source:expr $(, $pat:pat => $cond:expr)*) => {{
        let mut diagnostics = Default::default();
        let _ = $crate::tests::compile_helper($source, &mut diagnostics).expect("source should compile");
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

mod binary;
mod bug_326;
mod bug_344;
mod bug_417;
mod bug_422;
mod bug_428;
mod bug_454;
mod bugfixes;
mod char;
mod collections;
mod comments;
mod compiler_attributes;
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
mod core_macros;
mod custom_macros;
mod destructuring;
mod external_enum;
mod external_ops;
mod for_loop;
mod generic_native;
mod generics;
mod getter_setter;
mod instance;
mod iterator;
mod match_external;
mod moved;
mod patterns;
mod reference_error;
mod stmt_reordering;
mod test_attribute;
mod test_continue;
mod test_float;
mod test_int;
mod test_iter;
mod test_option;
mod test_quote;
mod test_range;
mod test_result;
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
