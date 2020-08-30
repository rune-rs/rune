use rune_testing::*;

#[test]
fn test_add() {
    assert_eq! {
        test! {
            i64 => r#"
            fn main() {
                let a = 10;
                let b = 2;
                a + b
            }
            "#
        },
        12,
    };

    assert_eq! {
        test! {
            i64 => r#"
            fn main() {
                let a = 10;
                let b = 2;
                a += b;
                a
            }
            "#
        },
        12,
    };

    test_vm_error!(
        r#"
        fn main() {
            let a = 9223372036854775807;
            let b = 2;
            a += b;
        }
        "#,
        Overflow => {}
    );

    test_vm_error!(
        r#"
        fn main() {
            let a = 9223372036854775807;
            let b = 2;
            a + b;
        }
        "#,
        Overflow => {}
    );
}

#[test]
fn test_sub() {
    assert_eq! {
        test! {
            i64 => r#"
            fn main() {
                let a = 10;
                let b = 2;
                a - b
            }
            "#
        },
        8,
    };

    assert_eq! {
        test! {
            i64 => r#"
            fn main() {
                let a = 10;
                let b = 2;
                a -= b;
                a
            }
            "#
        },
        8,
    };

    test_vm_error!(
        r#"
        fn main() {
            let a = -9223372036854775808;
            let b = 2;
            a -= b;
        }
        "#,
        Underflow => {}
    );

    test_vm_error!(
        r#"
        fn main() {
            let a = -9223372036854775808;
            let b = 2;
            a - b;
        }
        "#,
        Underflow => {}
    );
}

#[test]
fn test_mul() {
    assert_eq! {
        test! {
            i64 => r#"
            fn main() {
                let a = 10;
                let b = 2;
                a * b
            }
            "#
        },
        20,
    };

    assert_eq! {
        test! {
            i64 => r#"
            fn main() {
                let a = 10;
                let b = 2;
                a *= b;
                a
            }
            "#
        },
        20,
    };

    test_vm_error!(
        r#"
        fn main() {
            let a = 9223372036854775807;
            let b = 2;
            a *= b;
        }
        "#,
        Overflow => {}
    );

    test_vm_error!(
        r#"
        fn main() {
            let a = 9223372036854775807;
            let b = 2;
            a * b;
        }
        "#,
        Overflow => {}
    );
}

#[test]
fn test_div() {
    assert_eq! {
        test! {
            i64 => r#"
            fn main() {
                let a = 10;
                let b = 2;
                a / b
            }
            "#
        },
        5,
    };

    assert_eq! {
        test! {
            i64 => r#"
            fn main() {
                let a = 10;
                let b = 2;
                a /= b;
                a
            }
            "#
        },
        5,
    };

    test_vm_error!(
        r#"
        fn main() {
            let a = 10;
            let b = 0;
            a /= b;
        }
        "#,
        DivideByZero => {}
    );

    test_vm_error!(
        r#"
        fn main() {
            let a = 10;
            let b = 0;
            let c = a / b;
        }
        "#,
        DivideByZero => {}
    );
}
