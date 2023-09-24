prelude!();

use ErrorKind::*;

#[test]
fn illegal_pattern_in_match() -> rune::support::Result<()> {
    assert_errors! {
        r#"
        struct Foo { bar, baz }

        pub fn main() {
            match () { Foo { } => {} }
        }
        "#,
        span!(81, 88), PatternMissingFields { fields, .. } => {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].as_ref(), "bar");
            assert_eq!(fields[1].as_ref(), "baz");
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
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].as_ref(), "baz");
        }
    };

    Ok(())
}
