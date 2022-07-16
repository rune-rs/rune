use rune::query::QueryErrorKind::*;
use rune::span;
use rune_tests::*;

/// This tests that all items can be successfully queried for when unused (but
/// be ambiguous as is the case with `Foo::Variant`) and that a module with the
/// same name as an item causes a meta conflict.
#[test]
fn ensure_unambigious_items() {
    assert_errors! {
        r#"enum Foo { Variant } mod Foo { struct Variant; }"#,
        span,
        QueryError(MetaConflict { .. }) => {
            assert_eq!(span, span!(0, 20));
        },
        QueryError(AmbiguousItem { .. }) => {
            assert_eq!(span, span!(11, 18));
        },
    };
}
