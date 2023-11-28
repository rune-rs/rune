#![allow(clippy::unit_cmp)]

prelude!();

#[test]
fn test_small_programs() {
    let out: u64 = rune!(
        pub fn main() {
            42
        }
    );
    assert_eq!(out, 42u64);
    rune!(
        pub fn main() {}
    );

    let out: i64 = rune! {
        pub fn main() {
            let a = 1;
            let b = 2;
            let c = a + b;
            let d = c * 2;
            let e = d / 3;
            e
        }
    };
    assert_eq!(out, 2);
}

#[test]
fn test_boolean_ops() {
    let out: bool = rune!(
        pub fn main() {
            true && true
        }
    );
    assert_eq!(out, true);

    let out: bool = rune!(
        pub fn main() {
            true && false
        }
    );
    assert_eq!(out, false);

    let out: bool = rune!(
        pub fn main() {
            false && true
        }
    );
    assert_eq!(out, false);

    let out: bool = rune!(
        pub fn main() {
            false && false
        }
    );
    assert_eq!(out, false);

    let out: bool = rune!(
        pub fn main() {
            true || true
        }
    );
    assert_eq!(out, true);

    let out: bool = rune!(
        pub fn main() {
            true || false
        }
    );
    assert_eq!(out, true);

    let out: bool = rune!(
        pub fn main() {
            false || true
        }
    );
    assert_eq!(out, true);

    let out: bool = rune!(
        pub fn main() {
            false || false
        }
    );
    assert_eq!(out, false);
}

#[test]
fn test_if() {
    let out: i64 = rune! {
        pub fn main() {
            let n = 2;

            if n > 5 {
                10
            } else {
                0
            }
        }
    };
    assert_eq!(out, 0);

    let out: i64 = rune! {
        pub fn main() {
            let n = 6;

            if n > 5 {
                10
            } else {
                0
            }
        }
    };
    assert_eq!(out, 10);
}

#[test]
fn test_block() {
    let out: i64 = rune! {
        pub fn main() {
            let b = 10;

            let n = {
                let a = 10;
                a + b
            };

            n + 1
        }
    };
    assert_eq!(out, 21);
}

#[test]
fn test_shadowing() {
    let out: i64 = rune! {
        pub fn main() {
            let a = 10;
            let a = a;
            a
        }
    };
    assert_eq!(out, 10);
}

#[test]
fn test_vectors() {
    rune!(
        pub fn main() {
            let v = [1, 2, 3, 4, 5];
        }
    );
}

#[test]
fn test_while() {
    let out: i64 = rune! {
        pub fn main() {
            let a = 0;

            while a < 10 {
                a = a + 1;
            }

            a
        }
    };
    assert_eq!(out, 10);

    let out: i64 = rune! {
        pub fn main() {
            let a = 0;

            let a = while a >= 0 {
                if a >= 10 {
                    break a;
                }

                a = a + 1;
            };

            a
        }
    };
    assert_eq!(out, 10);
}

#[test]
fn test_loop() {
    let out: VecTuple<(i64, bool)> = rune! {
        pub fn main() {
            let a = 0;

            let value = loop {
                if a >= 10 {
                    break;
                }

                a = a + 1;
            };

            [a, value is Tuple]
        }
    };
    assert_eq!(out, VecTuple((10, true)));

    let out: i64 = rune! {
        pub fn main() {
            let n = 0;

            let n = loop {
                if n >= 10 {
                    break n;
                }

                n = n + 1;
            };

            n
        }
    };
    assert_eq!(out, 10);
}

#[test]
fn test_for() {
    let out: i64 = rune! {
        use std::iter::range;

        pub fn main() {
            let a = 0;
            let it = range(0, 10);

            for v in it {
                a = a + 1;
            }

            a
        }
    };
    assert_eq!(out, 10);

    let out: i64 = rune! {
        use std::iter::range;

        pub fn main() {
            let a = 0;
            let it = range(0, 100);

            let a = for v in it {
                if a >= 10 {
                    break a;
                }

                a = a + 1;
            };

            a
        }
    };
    assert_eq!(out, 10);

    let out: bool = rune! {
        use std::iter::range;

        pub fn main() {
            let a = 0;
            let it = range(0, 100);

            let a = for v in it {
                if a >= 10 {
                    break;
                }

                a = a + 1;
            };

            a is Tuple
        }
    };
    assert_eq!(out, true);
}

#[test]
fn test_return() {
    let out: i64 = rune! {
        use std::iter::range;

        pub fn main() {
            for v in range(0, 20) {
                if v == 10 {
                    return v;
                }
            }

            0
        }
    };
    assert_eq!(out, 10);
}

#[test]
fn test_is() {
    let out: bool = rune!(pub fn main() { {} is Object });
    assert!(!out);

    let out: bool = rune!(pub fn main() { #{} is Object });
    assert!(out);

    let out: bool = rune!(pub fn main() { () is Tuple });
    assert!(out);

    let out: bool = rune!(fn foo() {} pub fn main() { foo() is Tuple });
    assert!(out);

    let out: bool = rune!(pub fn main() {{} is Tuple });
    assert!(out);

    let out: bool = rune!(pub fn main() { true is bool });
    assert!(out);

    let out: bool = rune!(pub fn main() { 'a' is char });
    assert!(out);

    let out: bool = rune!(pub fn main() { 42u8 is u8 });
    assert!(out);

    let out: bool = rune!(pub fn main() { 42 is i64 });
    assert!(out);

    let out: bool = rune!(pub fn main() { 42.1 is f64 });
    assert!(out);

    let out: bool = rune!(pub fn main() { "hello" is String });
    assert!(out);

    let out: bool = rune!(pub fn main() { #{"hello": "world"} is Object });
    assert!(out);

    let out: bool = rune!(pub fn main() { ["hello", "world"] is Vec });
    assert!(out);
}

#[test]
fn test_destructuring() {
    let out: i64 = rune! {
        fn foo(n) {
            [n, n + 1]
        }

        pub fn main() {
            let [a, b] = foo(3);
            a + b
        }
    };
    assert_eq!(out, 7);
}

#[test]
fn test_if_pattern() {
    let out: bool = rune! {
        pub fn main() {
            if let [value] = [()] {
                true
            } else {
                false
            }
        }
    };
    assert_eq!(out, true);

    let out: bool = rune! {
        pub fn main() {
            if let [value] = [(), ()] {
                true
            } else {
                false
            }
        }
    };
    assert_eq!(out, false);

    let out: i64 = rune! {
        pub fn main() {
            let value = [(), (), 2];

            if let [(), ()] = value {
                1
            } else if let [(), (), c] = value {
                c
            } else {
                3
            }
        }
    };
    assert_eq!(out, 2);
}

#[test]
fn test_break_label() {
    let out: i64 = rune! {
        use std::iter::range;

        pub fn main() {
            let it = range(0, 1000);
            let tail = 77;

            'label:
            while true {
                let value = 10;

                for n in it {
                    loop {
                        let value2 = 20;
                        break 'label;
                    }

                    tail = tail + 1;
                }

                tail = tail + 1;
            }

            tail
        }
    };
    assert_eq!(out, 77);
}

#[test]
fn test_string_concat() {
    let out: String = rune! {
        pub fn main() {
            let foo = String::from("foo");
            foo += "/bar" + "/baz";
            foo
        }
    };
    assert_eq!(out, "foo/bar/baz");
}

#[test]
fn test_template_string() {
    let out: String = rune_s! { r#"
        pub fn main() {
            let name = "John Doe";
            `Hello ${name}, I am ${1 - 10} years old!`
        }
    "# };
    assert_eq!(out, "Hello John Doe, I am -9 years old!");

    // Contrived expression with an arbitrary sub-expression.
    // This tests that the temporary variables used during calculations do not
    // accidentally clobber the scope.
    let out: String = rune_s! { r#"
        pub fn main() {
            let name = "John Doe";

            `Hello ${name}, I am ${{
                let a = 20;
                a += 2;
                a
            }} years old!`
        }
    "# };
    assert_eq!(out, "Hello John Doe, I am 22 years old!");
}

#[test]
fn test_variants_as_functions() {
    let out: i64 = rune! {
        enum Foo { A(a), B(b, c) }

        fn construct_tuple(tuple) {
            tuple(1, 2)
        }

        pub fn main() {
            let foo = construct_tuple(Foo::B);

            match foo {
                Foo::B(a, b) => a + b,
                _ => 0,
            }
        }
    };
    assert_eq!(out, 3);
}

#[test]
fn test_iter_drop() {
    let out: i64 = rune! {
        pub fn main() {
            let sum = 0;
            let values = [1, 2, 3, 4];

            for v in values.iter() {
                break;
            }

            values.push(5);

            for v in values.iter() {
                sum += v;

                if v == 2 {
                    break;
                }
            }

            values.push(6);

            for v in values.iter() {
                sum += v;
            }

            sum
        }
    };
    assert_eq!(out, 24);
}

#[test]
fn test_async_fn() {
    let out: i64 = rune! {
        async fn foo(a, b) {
            b / a
        }

        fn bar(a, b) {
            b / a
        }

        pub async fn main() {
            foo(2, 4).await + bar(2, 8)
        }
    };
    assert_eq!(out, 6);
}

#[test]
fn test_complex_field_access() {
    let out: Option<i64> = rune_s! { r#"
        fn foo() {
            #{hello: #{world: 42}}
        }

        pub fn main() {
            Some((foo()).hello["world"])
        }
    "# };
    assert_eq!(out, Some(42));
}

#[test]
fn test_index_get() {
    let out: i64 = rune! {
        struct Named(a, b, c);
        enum Enum { Named(a, b, c) }

        fn a() { [1, 2, 3] }
        fn b() { (2, 3, 4) }
        fn c() { Named(3, 4, 5) }
        fn d() { Enum::Named(4, 5, 6) }

        pub fn main() {
            (a())[1] + (b())[1] + (c())[1] + (d())[1] + (a()).2 + (b()).2 + (c()).2 + (d()).2
        }
    };
    assert_eq!(out, 32);
}
