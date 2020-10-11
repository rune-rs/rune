#[test]
fn test_lazy_and_or() {
    assert_eq! {
        rune!(bool => pub fn main() { true || return false }),
        true,
    };

    assert_eq! {
        rune!(bool => pub fn main() { false && return true }),
        false,
    };

    assert_eq! {
        rune!(bool => pub fn main() { false || false || {return true; false} || false }),
        true,
    };

    assert_eq! {
        rune!(bool => pub fn main() { false && false && {return false; false} || true }),
        true,
    };
}
