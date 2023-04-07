prelude!();

#[test]
fn test_option() {
    let out: i64 = rune! {
        pub fn main() { match Some("some") { Some("some") => 1,  _ => 2 } }
    };
    assert_eq!(out, 1);

    let out: i64 = rune! {
        pub fn main() { match Some("some") { Some("other") => 1,  _ => 2 } }
    };
    assert_eq!(out, 2);

    let out: i64 = rune! {
        pub fn main() { match None { None => 1,  _ => 2 } }
    };
    assert_eq!(out, 1);

    let out: i64 = rune! {
        pub fn main() { match None { Some("some") => 1,  _ => 2 } }
    };
    assert_eq!(out, 2);
}
