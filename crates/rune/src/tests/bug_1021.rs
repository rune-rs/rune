prelude!();

/// A block-like expression in statement position followed by a parenthesized
/// expression was treated as a call, so `if true { } (1, 2, 3);` called the
/// unit value of the `if` instead of being parsed as two statements.
///
/// See https://github.com/rune-rs/rune/issues/1021
#[test]
fn test_bug_1021() {
    let _: () = rune! {
        pub fn main() {
            if true { }
            (1, 2, 3);
        }

        main()
    };

    let _: () = rune! {
        pub fn main() {
            if true { 1 } else { 0 }
            (1, 2, 3);
        }

        main()
    };

    // In expression position a block-like expression can still be called.
    let out: i64 = rune! {
        let f = if true { || 1 } else { || 2 }();
        f
    };

    assert_eq!(out, 1);
}
