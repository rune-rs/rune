#[test]
fn test_lazy_and_or() {
    assert_eq! {
        rune!(bool => r#"fn main() { true || return false }"#),
        true,
    };

    assert_eq! {
        rune!(bool => r#"fn main() { false && return true }"#),
        false,
    };

    assert_eq! {
        rune!(bool => r#"fn main() { false || false || {return true; false} || false }"#),
        true,
    };

    assert_eq! {
        rune!(bool => r#"fn main() { false && false && {return false; false} || true }"#),
        true,
    };
}
