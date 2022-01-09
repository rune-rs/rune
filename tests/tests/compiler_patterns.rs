use rune::compile::CompileErrorKind::*;
use rune::span;
use rune_tests::*;

#[test]
fn illegal_pattern_in_match() {
    assert_compile_error! {
        r#"
        struct Foo { bar, baz }

        pub fn main() {
            match () { Foo { } => {} }
        }
        "#,
        span, PatternMissingFields { fields, .. } => {
            assert_eq!(&fields[..], [Box::from("bar"), Box::from("baz")]);
            assert_eq!(span, span!(85, 88));
        }
    };

    assert_compile_error! {
        r#"
        struct Foo { bar, baz }

        pub fn main() {
            match () { Foo { bar } => {} }
        }
        "#,
        span, PatternMissingFields { fields, .. } => {
            assert_eq!(&fields[..], [Box::from("baz")]);
            assert_eq!(span, span!(85, 92));
        }
    };
}
