prelude!();

use ErrorKind::*;

#[test]
fn impl_in_other_mod() {
    rune! {
        struct Foo;

        mod lol {
            use super::Foo;

            impl Foo {
                fn lol(self) {
                    2
                }
            }
        }

        pub fn main() {
            assert_eq!(Foo.lol(), 2);
        }
    }
}

#[test]
fn impl_in_super() {
    rune! {
        struct Foo;

        mod lol {
            impl super::Foo {
                fn lol(self) {
                    3
                }
            }
        }

        pub fn main() {
            assert_eq!(Foo.lol(), 3);
        }
    }
}

#[test]
fn impl_in_block() {
    rune! {
        struct Foo;

        pub fn main() {
            let value = {
                impl Foo {
                    fn lol(self) {
                        4
                    }
                }
            };

            assert_eq!(Foo.lol(), 4);
        }
    }
}

#[test]
fn deny_self_impl() {
    assert_errors! {
        r#"
        struct Foo;

        impl Foo {
            fn a() {
                impl Self {
                    fn b(self) {}
                }
            }
        }
        "#,
        span!(83, 87), UnsupportedSelfType,
    }
}
