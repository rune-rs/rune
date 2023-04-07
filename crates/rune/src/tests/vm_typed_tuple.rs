prelude!();

#[test]
fn test_defined_tuple() {
    let out: i64 = rune! {
        struct MyType(a, b);

        pub fn main() { match MyType(1, 2) { MyType(a, b) => a + b,  _ => 0 } }
    };
    assert_eq!(out, 3);

    let out: i64 = rune! {
        enum MyType { A(a, b), C(c), }

        pub fn main() { match MyType::A(1, 2) { MyType::A(a, b) => a + b,  _ => 0 } }
    };
    assert_eq!(out, 3);

    let out: i64 = rune! {
        enum MyType { A(a, b), C(c), }

        pub fn main() { match MyType::C(4) { MyType::A(a, b) => a + b,  _ => 0 } }
    };
    assert_eq!(out, 0);

    let out: i64 = rune! {
        enum MyType { A(a, b), C(c), }

        pub fn main() { match MyType::C(4) { MyType::C(a) => a,  _ => 0 } }
    };
    assert_eq!(out, 4);
}
