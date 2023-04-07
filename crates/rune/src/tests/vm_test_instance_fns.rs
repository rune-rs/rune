prelude!();

#[test]
fn test_instance_kinds() {
    let out: (i64, i64, i64, i64) = rune! {
        struct Foo {
            n,
        }

        impl Foo {
            fn test(self, n) {
                self.n + 1
            }
        }

        enum Custom {
            A(n),
            B {
                n
            },
            C,
        }

        impl Custom {
            fn test(self) {
                match self {
                    Custom::A(n) => n + 1,
                    Custom::B{n} => n + 1,
                    Custom::C => 7,
                }
            }
        }

        pub fn main() {
            (Foo { n: 3 }.test(1), Custom::A(4).test(), Custom::B{n: 5}.test(), Custom::C.test())
        }
    };

    assert_eq!(out, (4, 5, 6, 7));
}
