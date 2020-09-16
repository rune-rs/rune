#[test]
fn test_unwrap() {
    assert_eq! {
        rune! {
            Result<i64, i64> => r#"
            fn foo(a, b) {
                Ok(b / a)
            }

            fn bar(a, b) {
                Err(b / a)
            }

            fn main() {
                Ok(foo(2, 4)? + bar(3, 9)?)
            }
            "#
        },
        Err(3),
    };

    assert_eq! {
        rune! {
            Result<i64, i64> => r#"
            fn foo(a, b) {
                Ok(b / a)
            }

            fn main() {
                Ok(foo(2, 4)? + {
                    Err(6 / 2)
                }?)
            }
            "#
        },
        Err(3),
    };
}
