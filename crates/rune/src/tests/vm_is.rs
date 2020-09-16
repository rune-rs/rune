#[test]
fn test_binop_override() {
    // The right hand side of the `is` expression requires a type, and therefore
    // won't be used as an empty tuple constructor.
    assert_eq! {
        rune! {
            (bool, bool, bool, bool) => r#"
            struct Timeout;

            fn main() {
                let timeout = Timeout;

                (
                    timeout is Timeout,
                    timeout is not Timeout,
                    !(timeout is Timeout),
                    !(timeout is not Timeout),
                )
            }
            "#
        },
        (true, false, false, true),
    };
}
