prelude!();

#[test]
fn test_nested_mods() {
    let out: i64 = rune! {
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

        pub fn main() {
            hello::test()
        }
    };
    assert_eq!(out, 3);
}
