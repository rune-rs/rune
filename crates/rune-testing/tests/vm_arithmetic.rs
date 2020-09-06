use rune_testing::*;

#[test]
fn test_add() {
    assert_eq! {
        rune! {
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
        rune! {
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

    assert_vm_error!(
        r#"
        fn main() {
            let a = 9223372036854775807;
            let b = 2;
            a += b;
        }
        "#,
        Overflow => {}
    );

    assert_vm_error!(
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
        rune! {
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
        rune! {
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

    assert_vm_error!(
        r#"
        fn main() {
            let a = -9223372036854775808;
            let b = 2;
            a -= b;
        }
        "#,
        Underflow => {}
    );

    assert_vm_error!(
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
        rune! {
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
        rune! {
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

    assert_vm_error!(
        r#"
        fn main() {
            let a = 9223372036854775807;
            let b = 2;
            a *= b;
        }
        "#,
        Overflow => {}
    );

    assert_vm_error!(
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
        rune! {
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
        rune! {
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

    assert_vm_error!(
        r#"
        fn main() {
            let a = 10;
            let b = 0;
            a /= b;
        }
        "#,
        DivideByZero => {}
    );

    assert_vm_error!(
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

#[test]
fn test_rem() {
    assert_eq! {
        rune! {
            i64 => r#"
            fn main() {
                let a = 10;
                let b = 3;
                a % b
            }
            "#
        },
        1,
    };

    assert_eq! {
        rune! {
            i64 => r#"
            fn main() {
                let a = 10;
                let b = 3;
                a %= b;
                a
            }
            "#
        },
        1,
    };

    assert_vm_error!(
        r#"
        fn main() {
            let a = 10;
            let b = 0;
            a %= b;
        }
        "#,
        DivideByZero => {}
    );

    assert_vm_error!(
        r#"
        fn main() {
            let a = 10;
            let b = 0;
            let c = a % b;
        }
        "#,
        DivideByZero => {}
    );
}

#[test]
fn test_bit_ops() {
    assert_eq! {
        rune! {
            i64 => r#"
            fn main() { let a = 0b1100; let b = 0b0110; a & b }
            "#
        },
        0b1100 & 0b0110,
    };

    assert_eq! {
        rune! {
            i64 => r#"
            fn main() { let a = 0b1100; a &= 0b0110; a }
            "#
        },
        0b1100 & 0b0110,
    };

    assert_eq! {
        rune! {
            i64 => r#"
            fn main() { let a = 0b1100; let b = 0b0110; a ^ b }
            "#
        },
        0b1100 ^ 0b0110,
    };

    assert_eq! {
        rune! {
            i64 => r#"
            fn main() { let a = 0b1100; a ^= 0b0110; a }
            "#
        },
        0b1100 ^ 0b0110,
    };

    assert_eq! {
        rune! {
            i64 => r#"
            fn main() { let a = 0b1100; let b = 0b0110; a | b }
            "#
        },
        0b1100 | 0b0110,
    };

    assert_eq! {
        rune! {
            i64 => r#"
            fn main() { let a = 0b1100; a |= 0b0110; a }
            "#
        },
        0b1100 | 0b0110,
    };
}

#[test]
fn test_shift_ops() {
    assert_eq! {
        rune! {
            i64 => r#"
            fn main() { let a = 0b10100; let b = 2; a >> b }
            "#
        },
        0b101,
    };

    assert_eq! {
        rune! {
            i64 => r#"
            fn main() { let a = 0b10100; a >>= 2; a }
            "#
        },
        0b101,
    };

    assert_eq! {
        rune! {
            i64 => r#"
            fn main() { let a = 0b10100; let b = 2; a << b }
            "#
        },
        0b1010000,
    };

    assert_eq! {
        rune! {
            i64 => r#"
            fn main() { let a = 0b10100; a <<= 2; a }
            "#
        },
        0b1010000,
    };
}

#[test]
fn test_bitwise_not() {
    assert_eq! {
        rune! {
            i64 => r#"
            fn main() { let a = 0b10100; !a }
            "#
        },
        !0b10100,
    };
}
