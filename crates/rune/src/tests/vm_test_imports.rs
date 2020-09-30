#[test]
fn test_grouped_imports() {
    assert_eq! {
        rune! {
            (i64, bool, bool) => r#"
            use a::{b::*, b::Foo::Baz, c};

            pub mod a {
                pub mod b {
                    pub enum Foo { Bar, Baz, }
                }
            
                pub mod c {
                    pub const VALUE = 2;
                }
            }

            fn main() {
                (c::VALUE, Foo::Bar is a::b::Foo, Baz is a::b::Foo)
            }                     
            "#
        },
        (2, true, true),
    };
}
