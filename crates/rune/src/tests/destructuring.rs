prelude!();

macro_rules! test_case {
    (($($st:tt)*), ($($ds:tt)*) $(, $($extra:tt)*)?) => {
        let out: i64 = rune! {
            $($($extra)*)?

            fn foo($($ds)*) {
                a + b
            }

            pub fn main() {
                let n = 0;

                for (a, b) in [(1, 2), (2, 3), (3, 4)] {
                    n += foo($($st)*);
                }

                n
            }
        };
        assert_eq!(out, 15);

        let out: i64 = rune! {
            $($($extra)*)?

            pub fn main() {
                let foo = |$($ds)*| {
                    a + b
                };

                let n = 0;

                for (a, b) in [(1, 2), (2, 3), (3, 4)] {
                    n += foo($($st)*);
                }

                n
            }
        };
        assert_eq!(out, 15);
    }
}

#[test]
fn test_fn_destructuring() {
    test_case!((a, b), (a, b));
    test_case!(((a, b)), ((a, b)));
    test_case!((#{a, b}), (#{a, b}));
    test_case!((#{a, c: b}), (#{a, c: b}));
    test_case!((Foo { a, b }), (Foo { a, b }), struct Foo { a, b });
    test_case!((Foo { a, c: b }), (Foo { a, c: b }), struct Foo { a, c });
    test_case!((Foo(a, b)), (Foo(a, b)), struct Foo(a, b););
    test_case!((Foo::Var {a, b}), (Foo::Var {a, b}), enum Foo { Var{a, b} };);
    test_case!((Foo::Var(a, b)), (Foo::Var(a, b)), enum Foo { Var(a, b) };);
}
