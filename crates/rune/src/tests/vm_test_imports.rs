#[test]
fn test_grouped_imports() {
    assert_eq! {
        rune! {
            (i64, bool, bool) => r#"
            use a::{b::*, b::Foo::Baz, c};

            mod a {
                mod b {
                    enum Foo { Bar, Baz, }
                }
            
                mod c {
                    const VALUE = 2;
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
