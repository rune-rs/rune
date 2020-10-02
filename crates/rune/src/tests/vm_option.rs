#[test]
fn test_option() {
    assert_eq! {
        rune! { i64 =>
            fn main() { match Err("err") { Err("err") => 1,  _ => 2 } }
        },
        1,
    };

    assert_eq! {
        rune! { i64 =>
            fn main() { match Err("err") { Ok("ok") => 1,  _ => 2 } }
        },
        2,
    };

    assert_eq! {
        rune! { i64 =>
            fn main() { match Ok("ok") { Ok("ok") => 1,  _ => 2 } }
        },
        1,
    };
}
