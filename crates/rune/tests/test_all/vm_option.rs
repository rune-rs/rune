#[test]
fn test_option() {
    assert_eq! {
        rune! { i64 =>
            pub fn main() { match Some("some") { Some("some") => 1,  _ => 2 } }
        },
        1,
    };

    assert_eq! {
        rune! { i64 =>
            pub fn main() { match Some("some") { Some("other") => 1,  _ => 2 } }
        },
        2,
    };

    assert_eq! {
        rune! { i64 =>
            pub fn main() { match None { None => 1,  _ => 2 } }
        },
        1,
    };

    assert_eq! {
        rune! { i64 =>
            pub fn main() { match None { Some("some") => 1,  _ => 2 } }
        },
        2,
    };
}
