prelude!();

#[test]
fn test_basic_self() {
    rune! {
        struct Foo {
            value,
        }

        impl Foo {
            fn inc(self) {
                self.value += 1;
            }
        }

        pub fn main() {
            let foo = Foo { value: 42 };
            assert_eq!(foo.value, 42);
            foo.inc();
            assert_eq!(foo.value, 43);
        }
    };
}

#[test]
fn test_chaining() {
    rune! {
        struct Foo {
            value,
        }

        impl Foo {
            fn inc(self) {
                self.value += 1;
                self
            }
        }

        pub fn main() {
            let foo = Foo { value: 42 };
            assert_eq!(foo.inc().inc().inc().value, 45);
        }
    };
}
