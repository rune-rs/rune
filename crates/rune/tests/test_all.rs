/// Assert that the given parse error happens with the given rune program.
#[macro_export]
macro_rules! assert_parse_error {
    ($source:expr, $span:ident, $pat:pat => $cond:expr) => {{
        let context = std::sync::Arc::new(rune_modules::default_context().unwrap());
        let errors = rune::testing::compile_source(&context, &$source).unwrap_err();
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
        let e = rune::testing::run::<_, _, $ty>(&context, &["main"], (), $source).unwrap_err();

        let (e, _) = match e {
            rune::testing::RunError::VmError(e) => e.into_unwound(),
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
        rune::testing::compile_source(&context, $source).unwrap()
    }};
}

/// Assert that the given rune program raises a compile error.
macro_rules! assert_compile_error {
    ($source:expr, $span:ident, $pat:pat => $cond:expr) => {{
        let context = rune_modules::default_context().unwrap();
        let e = rune::testing::compile_source(&context, $source).unwrap_err();
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
        let (_, warnings) = rune::testing::compile_source(&context, $source).expect("source should compile");
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

/// Rune the given string as a rune program.
macro_rules! rune_s {
    ($ty:ty => $source:expr) => {{
        let context = rune_modules::default_context().expect("failed to build context");
        let context = std::sync::Arc::new(context);

        rune::testing::run::<_, (), $ty>(&context, &["main"], (), $source)
            .expect("program to run successfully")
    }};
}

/// Rune the given ast as a rune program.
macro_rules! rune {
    ($ty:ty => $($tt:tt)*) => {{
        let context = rune_modules::default_context().expect("failed to build context");
        let context = std::sync::Arc::new(context);

        rune::testing::run::<_, (), $ty>(&context, &["main"], (), stringify!($($tt)*))
            .expect("program to run successfully")
    }};
}

#[path = "test_all/mod.rs"]
mod inner;
