#[test]
fn test_nested_mods() {
    assert_eq! {
        3,
        rune! {
            i64 => r#"
            mod hello {
                mod inner {
                    fn test() {
                        2
                    }
                }

                fn test() {
                    1 + inner::test()
                }
            }

            fn main() {
                hello::test()
            }
            "#
        }
    };
}
