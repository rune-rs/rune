use rune_tests::*;

#[test]
fn test_lazy_and_or() {
    assert! {
        rune!(bool => pub fn main() { true || return false }),
    };

    assert! {
        !rune!(bool => pub fn main() { false && return true }),
    };

    assert! {
        rune!(bool => pub fn main() { false || false || {return true; false} || false }),
    };

    assert! {
        rune!(bool => pub fn main() { false && false && {return false; false} || true }),
    }
}
