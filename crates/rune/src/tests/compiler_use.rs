use crate::testing::*;

#[test]
fn test_import_cycle() {
    assert_compile_error! {
        r#"
        mod a {
            pub mod c { pub use super::b::Bar as Baz; }
            pub mod b { pub use super::c::Baz as Bar; }
            pub use self::b::Bar as Foo;
        }

        use self::a::Foo;

        fn main() {
            Foo
        }             
        "#,
        span, QueryError { error } => {
            assert_eq!(span, Span::new(240, 243));

            match *error {
                ImportCycle { .. } => (),
                other => panic!("unexpected query error: {:?}", other),
            }
        }
    };

    assert_compile_error! {
        r#"
        mod b {
            pub use super::a::Foo;
        }
        
        mod a {
            pub use super::b::Foo;
        }
        
        fn main() {
            a::Foo
        }           
        "#,
        span, QueryError { error } => {
            assert_eq!(span, Span::new(173, 179));

            let path = match *error {
                ImportCycle { path, .. } => path,
                other => panic!("unexpected query error: {:?}", other),
            };

            assert_eq!(2, path.len());
            assert_eq!(Span::new(107, 120), path[0].location.span);
            assert_eq!(Span::new(37, 50), path[1].location.span);
        }
    };
}

#[test]
fn test_recursive_import() {
    let result = rune! { bool =>
        mod a {
            pub mod c { pub struct Baz; }
            pub mod b { pub use super::c::Baz as Bar; }
            pub use self::b::Bar as Foo;
        }

        use self::a::Foo;

        fn main() {
            Foo is a::c::Baz
        }
    };

    assert!(result);
}

#[test]
fn test_recursive_context_import() {
    let result = rune! { bool =>
        mod a {
            pub mod c { pub use std::option::Option as Baz; }
            pub mod b { pub use super::c::Baz as Bar; }
            pub use self::b::Bar as Foo;
        }

        use self::a::Foo;

        fn main() {
            Foo::None is Option
        }
    };

    assert!(result);
}

#[test]
fn test_recusive_wildcard() {
    let result = rune! { (bool, bool) =>
        mod a {
            pub mod c { pub use std::option::Option as Baz; }
            pub mod b { pub use super::c::Baz as Bar; }
            pub use self::b::{Bar as Foo, Bar as Foo2};
        }

        use self::a::*;

        fn main() {
            (Foo::None is Option, Foo2::Some(2) is Option)
        }
    };

    assert_eq!(result, (true, true));
}

#[test]
fn test_reexport_fn() {
    let result = rune! { i64 =>
        pub mod a {
            pub mod b {
                pub fn out(n) { n + A }
                const A = 1;
            }
        }

        mod b { pub use crate::{a::b::out, a}; }

        fn main() {
            b::out(2) + b::a::b::out(4)
        }
    };

    assert_eq!(result, 8);
}
