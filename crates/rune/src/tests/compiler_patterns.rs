prelude!();

use CompileErrorKind::*;

#[test]
fn illegal_pattern_in_match() {
    assert_errors! {
        r#"
        struct Foo { bar, baz }

        pub fn main() {
            match () { Foo { } => {} }
        }
        "#,
        span!(81, 88), PatternMissingFields { fields, .. } => {
            assert_eq!(&fields[..], [Box::from("bar"), Box::from("baz")]);
        }
    };

    assert_errors! {
        r#"
        struct Foo { bar, baz }

        pub fn main() {
            match () { Foo { bar } => {} }
        }
        "#,
        span!(81, 92), PatternMissingFields { fields, .. } => {
            assert_eq!(&fields[..], [Box::from("baz")]);
        }
    };
}
