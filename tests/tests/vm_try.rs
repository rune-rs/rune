use rune_tests::*;

#[test]
fn test_unwrap() {
    assert_eq! {
        rune! { Result<i64, i64> =>
            fn foo(a, b) {
                Ok(b / a)
            }

            fn bar(a, b) {
                Err(b / a)
            }

            pub fn main() {
                Ok(foo(2, 4)? + bar(3, 9)?)
            }
        },
        Err(3),
    };

    assert_eq! {
        rune! { Option<bool> =>
            struct Bar {
                x,
                y,
            }

            pub fn main() {
                (Bar{ x: Some(1), y: None? }).x
            }
        },
        None,
    };

    assert_eq! {
        rune! { Result<i64, i64> =>
            fn foo(a, b) {
                Ok(b / a)
            }

            pub fn main() {
                Ok(foo(2, 4)? + {
                    Err(6 / 2)
                }?)
            }
        },
        Err(3),
    };
}
