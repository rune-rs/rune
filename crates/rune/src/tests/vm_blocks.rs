prelude!();

#[test]
fn test_anonymous_type_precedence() {
    let out: i64 = rune! {
        pub fn main() {
            fn a() { 1 }
            fn b() { return a(); fn a() { 2 } }
            a() + b()
        }
    };
    assert_eq!(out, 3);
}
