prelude!();

/// Tests that different variants of the same enum can be compared to each other
/// See: https://github.com/rune-rs/rune/pull/215
#[test]
fn assert_variant_comparisons() {
    rune! {
        enum Foo { A, B }

        pub fn main() {
            assert!(Foo::A != Foo::B);
            assert_eq!(Foo::A, Foo::A);
        }
    };

    rune! {
        enum Foo { A(a), B }

        pub fn main() {
            assert!(Foo::A(10) != Foo::B);
            assert_eq!(Foo::A(10), Foo::A(10));
        }
    };

    rune! {
        enum Foo { A { a }, B }

        pub fn main() {
            assert!(Foo::A { a: 10 } != Foo::B);
            assert_eq!(Foo::A { a: 10 }, Foo::A { a: 10 });
        }
    };
}
