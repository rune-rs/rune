prelude!();

use ErrorKind::*;

#[test]
fn illegal_pattern_in_match() -> rune::support::Result<()> {
    assert_errors! {
        r#"
        struct Foo { bar, baz }
        match () { Foo {} => {} }
        "#,
        span!(52, 58), PatternMissingFields { fields, .. } => {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].as_ref(), "bar");
            assert_eq!(fields[1].as_ref(), "baz");
        }
    };

    assert_errors! {
        r#"
        struct Foo { bar, baz }
        match () { Foo { bar } => {} }
        "#,
        span!(52, 63), PatternMissingFields { fields, .. } => {
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].as_ref(), "baz");
        }
    };

    Ok(())
}
