#[test]
fn test_nested_mods() {
    assert_eq! {
        3,
        rune! {
            i64 => r#"
            pub mod hello {
                pub mod inner {
                    pub fn test() {
                        2
                    }
                }

                pub fn test() {
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
