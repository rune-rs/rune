#![allow(clippy::mixed_case_hex_literals)]

prelude!();

/// This ensures that `e` literals found in hex number literals are not treated
/// as exponents.
#[test]
pub fn test_bug_422() {
    macro_rules! test_case {
        ($expr:expr) => {
            let value: u32 = rune! {
                pub fn main() { $expr }
            };

            assert_eq!(value, $expr);
        };
    }

    test_case!(0x40c61d + 1);
    test_case!(0x40c61f - 1);
    test_case!(0x40c61e);
    test_case!(0x40c61E);
    test_case!(0x40C61e);
}
