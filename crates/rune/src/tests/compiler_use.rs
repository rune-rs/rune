prelude!();

use CompileErrorKind::QueryError;
use QueryErrorKind::*;

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

        pub fn main() {
            Foo
        }             
        "#,
        span, QueryError { error: ImportCycle { .. } } => {
            assert_eq!(span, span!(244, 247));
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
        
        pub fn main() {
            a::Foo
        }           
        "#,
        span, QueryError { error: ImportCycle { path, .. } } => {
            assert_eq!(span, span!(177, 183));
            assert_eq!(3, path.len());
            assert_eq!(span!(107, 120), path[0].location.span);
            assert_eq!(span!(37, 50), path[1].location.span);
        }
    };
}

#[test]
fn test_recursive_import() {
    let result: bool = rune! {
        mod a {
            pub mod c { pub struct Baz; }
            pub mod b { pub use super::c::Baz as Bar; }
            pub use self::b::Bar as Foo;
        }

        use self::a::Foo;

        pub fn main() {
            Foo is a::c::Baz
        }
    };

    assert!(result);
}

#[test]
fn test_recursive_context_import() {
    let result: bool = rune! {
        mod a {
            pub mod c { pub use std::option::Option as Baz; }
            pub mod b { pub use super::c::Baz as Bar; }
            pub use self::b::Bar as Foo;
        }

        use self::a::Foo;

        pub fn main() {
            Foo::None is Option
        }
    };

    assert!(result);
}

#[test]
fn test_recusive_wildcard() {
    let result: (bool, bool) = rune! {
        mod a {
            pub mod c { pub use std::option::Option as Baz; }
            pub mod b { pub use super::c::Baz as Bar; }
            pub use self::b::{Bar as Foo, Bar as Foo2};
        }

        use self::a::*;

        pub fn main() {
            (Foo::None is Option, Foo2::Some(2) is Option)
        }
    };

    assert_eq!(result, (true, true));
}

#[test]
fn test_reexport_fn() {
    let result: i64 = rune! {
        pub mod a {
            pub mod b {
                pub fn out(n) { n + A }
                const A = 1;
            }
        }

        mod b { pub use crate::{a::b::out, a}; }

        pub fn main() {
            b::out(2) + b::a::b::out(4)
        }
    };

    assert_eq!(result, 8);
}
