#[macro_use]
extern crate rune_tests;

/// Assert that the given parse error happens with the given rune program.
#[macro_export]
macro_rules! assert_parse_error {
    ($source:expr, $span:ident, $pat:pat => $cond:expr) => {{
        let context = std::sync::Arc::new(rune_modules::default_context().unwrap());
        let errors = ::rune_tests::compile_source(&context, &$source).unwrap_err();
        let err = errors.into_iter().next().expect("expected one error");

        let e = match err.into_kind() {
            rune::ErrorKind::ParseError(e) => (e),
            kind => {
                panic!(
                    "expected parse error `{}` but was `{:?}`",
                    stringify!($pat),
                    kind
                );
            }
        };

        let $span = rune::Spanned::span(&e);

        match e.into_kind() {
            $pat => $cond,
            kind => {
                panic!("expected error `{}` but was `{:?}`", stringify!($pat), kind);
            }
        }
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
        let context = std::sync::Arc::new(rune_modules::default_context().unwrap());
        let e = ::rune_tests::run::<_, _, $ty>(&context, $source, &["main"], ()).unwrap_err();

        let (e, _) = match e {
            ::rune_tests::RunError::VmError(e) => e.into_unwound(),
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
        let context = rune_modules::default_context().unwrap();
        ::rune_tests::compile_source(&context, $source).unwrap()
    }};
}

/// Assert that the given rune program raises a compile error.
macro_rules! assert_compile_error {
    ($source:expr, $span:ident, $pat:pat => $cond:expr) => {{
        let context = rune_modules::default_context().unwrap();
        let e = ::rune_tests::compile_source(&context, $source).unwrap_err();
        let e = e.into_iter().next().expect("expected one error");

        let e = match e.into_kind() {
            rune::ErrorKind::CompileError(e) => (e),
            kind => {
                panic!(
                    "expected parse error `{}` but was `{:?}`",
                    stringify!($pat),
                    kind
                );
            }
        };

        let $span = rune::Spanned::span(&e);

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
macro_rules! assert_warnings {
    ($source:expr $(, $pat:pat => $cond:expr)*) => {{
        let context = rune_modules::default_context().unwrap();
        let (_, warnings) = ::rune_tests::compile_source(&context, $source).expect("source should compile");
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

/// Assert that the given value matches the provided pattern.
#[allow(unused)]
macro_rules! assert_matches {
    ($value:expr, $pat:pat) => {
        match $value {
            $pat => (),
            other => panic!("expected {}, but was {:?}", stringify!($pat), other),
        }
    };
}

mod collections;
mod compiler_attributes;
mod compiler_expr_assign;
mod compiler_expr_binary;
mod compiler_fn;
mod compiler_general;
mod compiler_literals;
mod compiler_paths;
mod compiler_use;
mod compiler_visibility;
mod compiler_warnings;
mod core_macros;
mod destructuring;
mod external_ops;
mod for_loop;
mod getter_setter;
mod iterator;
mod moved;
mod reference_error;
mod stmt_reordering;
mod test_continue;
mod test_iter;
mod test_option;
mod test_range;
mod test_result;
mod type_name_native;
mod type_name_rune;
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
