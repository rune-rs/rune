#[test]
fn test_path_type_match() {
    assert_eq! {
        rune! {
            bool => r#"
            enum Custom { A, B(a) }
            fn main() {
                match Custom::A { Custom::A => true, _ => false }
            }
            "#
        },
        true,
    };

    assert_eq! {
        rune! {
            bool => r#"
            enum Custom { A, B(a) }
            fn main() {
                match Custom::B(0) { Custom::A => true, _ => false }
            }
            "#
        },
        false,
    };

    assert_eq! {
        rune! {
            bool => r#"
            enum Custom { A, B(a) }
            fn main() {
                match Custom::B(0) { Custom::B(0) => true, _ => false }
            }
            "#
        },
        true,
    };

    assert_eq! {
        rune! {
            bool => r#"
            enum Custom { A, B { a } }
            fn main() {
                match (Custom::B { a: 0 }) { Custom::B { a: 0 } => true, _ => false }
            }
            "#
        },
        true,
    };

    assert_eq! {
        rune! {
            bool => r#"
            enum Custom { A, B { a } }
            fn test(a) { a == 0 }

            fn main() {
                match (Custom::B { a: 0 }) { Custom::B { a } if test(a) => true, _ => false }
            }
            "#
        },
        true,
    };
}

#[test]
fn test_struct_matching() {
    assert_eq! {
        rune! {
            i64 => r#"
            struct Foo { a, b }

            fn main() {
                let foo = Foo {
                    a: 1,
                    b: 2,
                };

                match foo {
                    Foo { a, b } => a + b,
                    _ => 0,
                }
            }
            "#
        },
        3,
    };

    assert_eq! {
        rune! {
            i64 => r#"
            struct Foo { a, b }

            fn main() {
                let b = 2;

                let foo = Foo {
                    a: 1,
                    b,
                };

                match foo {
                    Foo { a, b } => a + b,
                    _ => 0,
                }
            }
            "#
        },
        3,
    };
}
