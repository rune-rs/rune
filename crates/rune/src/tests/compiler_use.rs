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
