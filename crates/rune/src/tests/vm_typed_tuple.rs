#[test]
fn test_defined_tuple() {
    assert_eq! {
        rune! {
            i64 => r#"
            struct MyType(a, b);

            fn main() { match MyType(1, 2) { MyType(a, b) => a + b,  _ => 0 } }
            "#
        },
        3,
    };

    assert_eq! {
        rune! {
            i64 => r#"
            enum MyType { A(a, b), C(c), }

            fn main() { match MyType::A(1, 2) { MyType::A(a, b) => a + b,  _ => 0 } }
            "#
        },
        3,
    };

    assert_eq! {
        rune! {
            i64 => r#"
            enum MyType { A(a, b), C(c), }

            fn main() { match MyType::C(4) { MyType::A(a, b) => a + b,  _ => 0 } }
            "#
        },
        0,
    };

    assert_eq! {
        rune! {
            i64 => r#"
            enum MyType { A(a, b), C(c), }

            fn main() { match MyType::C(4) { MyType::C(a) => a,  _ => 0 } }
            "#
        },
        4,
    };
}
