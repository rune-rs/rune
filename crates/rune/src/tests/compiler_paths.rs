#[test]
fn test_super_self_crate_mod() {
    assert_eq! {
        rune! {
            i64 => r#"
            struct Foo;

            impl Foo {
                fn foo() { Self::bar() ^ 0b100000 }

                fn bar() { self::a::foo() ^ 0b10000 }
            }

            mod a {
                fn foo() { self::b::foo() ^ 0b1000 }

                mod b {
                    fn foo() { super::c::foo() ^ 0b100 }
                }

                mod c {
                    fn foo() { crate::root() ^ 0b10 }
                }
            }

            fn root() { 0b1 }

            fn main() { Foo::foo() }
            "#
        },
        0b111111,
    };
}

#[test]
fn test_super_use() {
    assert_eq! {
        rune! {
            i64 => r#"
            mod x {
                mod y {
                    fn foo() {
                        use crate::VALUE as A;
                        use super::VALUE as B;
                        A + B
                    }
                }

                const VALUE = 2;
            }

            const VALUE = 1;

            fn main() { x::y::foo() }
            "#
        },
        3,
    };
}
