#[test]
fn test_anonymous_type_precedence() {
    assert_eq! {
        3,
        rune! { i64 =>
            fn main() {
                fn a() { 1 }
                fn b() { return a(); fn a() { 2 } }
                a() + b()
            }
        }
    };
}
