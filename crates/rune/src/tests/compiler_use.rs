prelude!();

use ErrorKind::*;

/// Regression test: importing a context-installed module whose item path is a
/// single root component (e.g. `::http`) with `use http;` must not produce a
/// bogus `Cycle in import` error once the imported name is used.
#[test]
fn test_use_single_component_context_module() {
    let mut module = Module::with_crate("http").expect("failed to build module");
    module
        .function("meaning", || 42i64)
        .build()
        .expect("failed to register function");

    let mut context = Context::with_default_modules().expect("failed to build context");
    context.install(&module).expect("failed to install module");

    let out: i64 = run(
        &context,
        r#"
        use http;

        pub fn main() {
            http::meaning()
        }
        "#,
        (),
        false,
    )
    .expect("program failed to run");

    assert_eq!(out, 42);
}

/// Same as above, but the import is never used. This must still compile.
#[test]
fn test_use_single_component_context_module_unused() {
    let mut module = Module::with_crate("http").expect("failed to build module");
    module
        .function("meaning", || 42i64)
        .build()
        .expect("failed to register function");

    let mut context = Context::with_default_modules().expect("failed to build context");
    context.install(&module).expect("failed to install module");

    let out: i64 = run(
        &context,
        r#"
        use http;

        pub fn main() {
            42
        }
        "#,
        (),
        false,
    )
    .expect("program failed to run");

    assert_eq!(out, 42);
}

fn context_with_http() -> Context {
    let mut module = Module::with_crate("http").expect("failed to build module");
    module
        .function("meaning", || 42i64)
        .build()
        .expect("failed to register function");

    let mut context = Context::with_default_modules().expect("failed to build context");
    context.install(&module).expect("failed to install module");
    context
}

/// A single-component import is self-referential whether or not anything of
/// that name exists, so the import itself must not be what satisfies the check
/// that its target exists. Without this an unused `use nonexistent;` compiles,
/// while `use nonexistent::thing;` is rightly a missing item.
/// The first fatal error compiling `source`, past any warnings which come
/// before it, such as the one an unused import always produces.
fn first_error(source: &str) -> (ast::Span, ErrorKind) {
    let mut diagnostics = Default::default();
    let _ = crate::tests::compile_helper(source, &mut diagnostics).unwrap_err();

    for diagnostic in diagnostics.into_diagnostics() {
        if let crate::diagnostics::Diagnostic::Fatal(e) = diagnostic {
            match e.into_kind() {
                crate::diagnostics::FatalDiagnosticKind::CompileError(e) => {
                    return (ast::Spanned::span(&e), e.into_kind());
                }
                kind => panic!("expected a compile error, got {kind:?}"),
            }
        }
    }

    panic!("expected a fatal diagnostic");
}

#[test]
fn an_unused_single_component_import_of_nothing_is_a_missing_item() {
    let (span, kind) = first_error(
        r#"
        use nonexistent;
        pub fn main() {}
        "#,
    );

    assert_eq!(span, span!(9, 24));
    assert!(
        matches!(kind, MissingItem { ref item } if item.to_string() == "nonexistent"),
        "{kind:?}"
    );

    let (span, kind) = first_error(
        r#"
        mod a { use nonexistent; }
        pub fn main() {}
        "#,
    );

    assert_eq!(span, span!(17, 32));
    assert!(matches!(kind, MissingItem { .. }), "{kind:?}");
}

/// Using a name brought in by a single-component import of nothing used to be
/// reported as an import cycle, which points at the wrong problem.
#[test]
fn a_used_single_component_import_of_nothing_is_a_missing_item() {
    assert_errors! {
        r#"
        use nonexistent;
        pub fn main() { nonexistent::foo() }
        "#,
        span!(50, 66), MissingItem { item, .. } => {
            assert_eq!(item.to_string(), "nonexistent::foo");
        }
    };
}

/// A context module imported by its single-component name from inside a
/// module resolves just as it does at the root.
#[test]
fn a_context_module_can_be_imported_inside_a_module() {
    let context = context_with_http();

    let out: i64 = run(
        &context,
        r#"
        mod a {
            use http;
            pub fn f() { http::meaning() }
        }

        pub fn main() {
            a::f()
        }
        "#,
        (),
        false,
    )
    .expect("program failed to run");

    assert_eq!(out, 42);
}

/// A real cycle through a single-component name is still a cycle, and is not
/// mistaken for a self-referential import of a context module.
#[test]
fn a_cycle_through_a_context_module_name_is_still_a_cycle() {
    assert_errors! {
        r#"
        mod a { pub use super::b::http; }
        mod b { pub use super::a::http; }
        pub fn main() { a::http::meaning() }
        "#,
        _, ImportCycle { .. }
    };
}

#[test]
fn test_import_cycle() {
    assert_errors! {
        r#"
        mod a {
            pub mod c { pub use super::b::Bar as Baz; }
            pub mod b { pub use super::c::Baz as Bar; }
            pub use self::b::Bar as Foo;
        }

        use self::a::Foo;
        "#,
        span!(41, 69), ImportCycle { .. }
    };

    assert_errors! {
        r#"
        mod b {
            pub use super::a::Foo;
        }

        mod a {
            pub use super::b::Foo;
        }

        pub fn main() {
            a::Foo
        }           
        "#,
        span!(161, 167), ImportCycle { path, .. } => {
            assert_eq!(3, path.len());
            assert_eq!(span!(91, 112), path[0].location.span);
            assert_eq!(span!(29, 50), path[1].location.span);
        }
    };
}
