prelude!();

#[test]
fn local_assignments() {
    let _: () = rune! {
        pub fn main() {
            let n = 0;

            while n < 10 {
                n += 1;
            }

            assert_eq!(n, 10);
        }
    };
}

#[test]
fn call_function() {
    let _: () = rune! {
        fn foo(v) {
            v
        }

        pub fn main() {
            assert_eq!(foo(42), 42);
            assert_ne!(foo(42), 43);
        }
    };
}

#[test]
fn instance() {
    let _: () = rune! {
        struct Foo {
            n,
        }

        impl Foo {
            fn test(self, n) {
                self.n + n
            }
        }

        pub fn main() {
            let foo = Foo { n: 42 };
            assert_eq!(foo.test(10), 52);
        }
    };
}

#[test]
fn generator() {
    let _: () = rune! {
        fn foo() {
            yield 10;
            yield 20;
        }

        pub fn main() {
            let n = 0;

            for v in foo() {
                n += v;
            }

            assert_eq!(n, 30);
        }
    };
}
