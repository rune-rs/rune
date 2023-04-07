prelude!();

#[test]
fn test_binop_override() {
    // The right hand side of the `is` expression requires a type, and therefore
    // won't be used as an empty tuple constructor.
    let out: (bool, bool, bool, bool) = rune! {
        struct Timeout;

        pub fn main() {
            let timeout = Timeout;

            (
                timeout is Timeout,
                timeout is not Timeout,
                !(timeout is Timeout),
                !(timeout is not Timeout),
            )
        }
    };
    assert_eq!(out, (true, false, false, true));
}
