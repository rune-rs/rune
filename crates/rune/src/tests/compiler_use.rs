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
        span, ImportCycle { .. } => {
            assert_eq!(span, Span::new(240, 243));
        }
    };
}

#[test]
fn test_recursive_import() {
    let result = rune! {
        bool => r#"
        mod a {
            pub mod c { pub struct Baz; }
            pub mod b { pub use super::c::Baz as Bar; }
            pub use self::b::Bar as Foo;
        }

        use self::a::Foo;

        fn main() {
            Foo is a::c::Baz
        }                    
        "#
    };

    assert!(result);
}

#[test]
fn test_recursive_context_import() {
    let result = rune! {
        bool => r#"
        mod a {
            pub mod c { pub use std::option::Option as Baz; }
            pub mod b { pub use super::c::Baz as Bar; }
            pub use self::b::Bar as Foo;
        }

        use self::a::Foo;

        fn main() {
            Foo::None is Option
        }                
        "#
    };

    assert!(result);
}

#[test]
fn test_recusive_wildcard() {
    let result = rune! {
        (bool, bool) => r#"
        mod a {
            pub mod c { pub use std::option::Option as Baz; }
            pub mod b { pub use super::c::Baz as Bar; }
            pub use self::b::{Bar as Foo, Bar as Foo2};
        }
        
        use self::a::*;
        
        fn main() {
            (Foo::None is Option, Foo2::Some(2) is Option)
        }             
        "#
    };

    assert_eq!(result, (true, true));
}
