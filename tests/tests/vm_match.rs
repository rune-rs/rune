use rune_tests::*;

#[test]
fn test_path_type_match() {
    assert! {
        rune! { bool =>
            enum Custom { A, B(a) }
            pub fn main() {
                match Custom::A { Custom::A => true, _ => false }
            }
        },
    };

    assert! {
        !rune! { bool =>
            enum Custom { A, B(a) }
            pub fn main() {
                match Custom::B(0) { Custom::A => true, _ => false }
            }
        },
    };

    assert! {
        rune! { bool =>
            enum Custom { A, B(a) }
            pub fn main() {
                match Custom::B(0) { Custom::B(0) => true, _ => false }
            }
        },
    };

    assert! {
        rune! { bool =>
            enum Custom { A, B { a } }
            pub fn main() {
                match (Custom::B { a: 0 }) { Custom::B { a: 0 } => true, _ => false }
            }
        },
    };

    assert! {
        rune! { bool =>
            enum Custom { A, B { a } }
            fn test(a) { a == 0 }

            pub fn main() {
                match (Custom::B { a: 0 }) { Custom::B { a } if test(a) => true, _ => false }
            }
        },
    };
}

#[test]
fn test_struct_matching() {
    assert_eq! {
        rune! { i64 =>
            struct Foo { a, b }

            pub fn main() {
                let foo = Foo {
                    a: 1,
                    b: 2,
                };

                match foo {
                    Foo { a, b } => a + b,
                    _ => 0,
                }
            }
        },
        3,
    };

    assert_eq! {
        rune! { i64 =>
            struct Foo { a, b }

            pub fn main() {
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
        },
        3,
    };
}
