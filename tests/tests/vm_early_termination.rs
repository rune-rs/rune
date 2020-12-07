use rune_tests::*;

macro_rules! test_case {
    (($($k:tt)*), $field:tt, $index:tt, $($extra:tt)*) => {
        assert_eq! {
            rune!(bool => pub fn main() { let m = $($k)*; m[return true]; false } $($extra)*),
            true,
        };

        assert_eq! {
            rune!(bool => pub fn main() { let m = $($k)*; m[return true] = 0; false } $($extra)*),
            true,
        };

        assert_eq! {
            rune!(bool => pub fn main() { let m = $($k)*; m[$index] = return true; false } $($extra)*),
            true,
        };

        assert_eq! {
            rune!(bool => pub fn main() { let m = $($k)*; m.$field = return true; false } $($extra)*),
            true,
        };

        assert_eq! {
            rune!(bool => pub fn main() { $($k)*[return true]; false } $($extra)*),
            true,
        };

        assert_eq! {
            rune!(bool => pub fn main() { $($k)*[return true] = 0; false } $($extra)*),
            true,
        };

        assert_eq! {
            rune!(bool => pub fn main() { $($k)*[$index] = return true; false } $($extra)*),
            true,
        };

        assert_eq! {
            rune!(bool => pub fn main() { $($k)*.$field = return true; false } $($extra)*),
            true,
        };
    };

    (($($k:tt)*), $field:tt, $index:tt) => {
        test_case!(($($k)*), $field, $index,)
    };
}

#[test]
fn test_object_like_early_term() {
    test_case!(( #{} ), test, "test");
}

#[test]
fn test_tuple_like_early_term() {
    test_case!((()), 0, 0);
}

#[test]
fn test_typed_object_early_term() {
    test_case!((Foo()), 0, 0, struct Foo(););
}

#[test]
fn test_typed_tuple_early_term() {
    test_case!(( Foo { test: 0 } ), test, "test", struct Foo { test };);
}
