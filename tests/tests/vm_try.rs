use rune_tests::prelude::*;

#[test]
fn test_unwrap() {
    let out: Result<i64, i64> = rune! {
        fn foo(a, b) {
            Ok(b / a)
        }

        fn bar(a, b) {
            Err(b / a)
        }

        pub fn main() {
            Ok(foo(2, 4)? + bar(3, 9)?)
        }
    };
    assert_eq!(out, Err(3));

    let out: Option<bool> = rune! {
        struct Bar {
            x,
            y,
        }

        pub fn main() {
            (Bar{ x: Some(1), y: None? }).x
        }
    };
    assert_eq!(out, None);

    let out: Result<i64, i64> = rune! {
        fn foo(a, b) {
            Ok(b / a)
        }

        pub fn main() {
            Ok(foo(2, 4)? + {
                Err(6 / 2)
            }?)
        }
    };
    assert_eq!(out, Err(3));
}
