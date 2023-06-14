prelude!();

use ErrorKind::*;

/// This tests that all items can be successfully queried for when unused (but
/// be ambiguous as is the case with `Foo::Variant`) and that a module with the
/// same name as an item causes a meta conflict.
#[test]
fn ensure_unambigious_items() {
    assert_errors! {
        r#"enum Foo { Variant } mod Foo { struct Variant; }"#,
        span,
        _ => {
            assert_eq!(span, span!(21, 28));
        },
        AmbiguousItem { .. } => {
            assert_eq!(span, span!(11, 18));
        },
    };
}
