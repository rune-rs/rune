prelude!();

#[test]
fn test_lazy_and_or() {
    let result: bool = rune!(
        pub fn main() {
            true || return false
        }
    );
    assert_eq!(result, true);

    let result: bool = rune!(
        pub fn main() {
            false && return true
        }
    );
    assert_eq!(result, false);

    let result: bool = rune!(
        pub fn main() {
            false
                || false
                || {
                    return true;
                    false
                }
                || false
        }
    );
    assert_eq!(result, true);

    let result: bool = rune!(
        pub fn main() {
            false && false && {
                return false;
                false
            } || true
        }
    );
    assert_eq!(result, true);
}
