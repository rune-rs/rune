prelude!();

use ErrorKind::*;

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
        span!(49, 69), ImportCycle { .. }
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
            assert_eq!(span!(99, 112), path[0].location.span);
            assert_eq!(span!(37, 50), path[1].location.span);
        }
    };
}
