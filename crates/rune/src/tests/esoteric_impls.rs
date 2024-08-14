prelude!();

use ErrorKind::*;

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
