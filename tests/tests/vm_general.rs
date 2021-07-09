#![allow(clippy::unit_cmp)]

use rune_tests::*;

#[test]
fn test_small_programs() {
    assert_eq!(rune!(u64 => pub fn main() { 42 }), 42u64);
    assert_eq!(rune!(() => pub fn main() {}), ());

    assert_eq! {
        rune! { i64 =>
            pub fn main() {
                let a = 1;
                let b = 2;
                let c = a + b;
                let d = c * 2;
                let e = d / 3;
                e
            }
        },
        2,
    };
}

#[test]
fn test_boolean_ops() {
    assert! {
        rune!(bool => pub fn main() { true && true }),
    };

    assert! {
        !rune!(bool => pub fn main() { true && false }),
    };

    assert! {
        !rune!(bool => pub fn main() { false && true }),
    };

    assert! {
        !rune!(bool => pub fn main() { false && false }),
    };

    assert! {
        rune!(bool => pub fn main() { true || true }),
    };

    assert! {
        rune!(bool => pub fn main() { true || false }),
    };

    assert! {
        rune!(bool => pub fn main() { false || true }),
    };

    assert! {
        !rune!(bool => pub fn main() { false || false }),
    };
}

#[test]
fn test_if() {
    assert_eq! {
        rune! { i64 =>
            pub fn main() {
                let n = 2;

                if n > 5 {
                    10
                } else {
                    0
                }
            }
        },
        0,
    };

    assert_eq! {
        rune!{ i64 =>
            pub fn main() {
                let n = 6;

                if n > 5 {
                    10
                } else {
                    0
                }
            }
        },
        10,
    };
}

#[test]
fn test_block() {
    assert_eq! {
        rune! { i64 =>
            pub fn main() {
                let b = 10;

                let n = {
                    let a = 10;
                    a + b
                };

                n + 1
            }
        },
        21,
    };
}

#[test]
fn test_shadowing() {
    assert_eq! {
        rune! { i64 =>
            pub fn main() {
                let a = 10;
                let a = a;
                a
            }
        },
        10,
    };
}

#[test]
fn test_vectors() {
    assert_eq! {
        rune!(() => pub fn main() { let v = [1, 2, 3, 4, 5]; }),
        (),
    };
}

#[test]
fn test_while() {
    assert_eq! {
        rune!{ i64 =>
            pub fn main() {
                let a = 0;

                while a < 10 {
                    a = a + 1;
                }

                a
            }
        },
        10,
    };

    assert_eq! {
        rune! { i64 =>
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
        },
        10,
    };
}

#[test]
fn test_loop() {
    assert_eq! {
        rune! {
            runestick::VecTuple<(i64, bool)> =>
            pub fn main() {
                let a = 0;

                let value = loop {
                    if a >= 10 {
                        break;
                    }

                    a = a + 1;
                };

                [a, value is unit]
            }
        },
        runestick::VecTuple((10, true)),
    };

    assert_eq! {
        rune! { i64 =>
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
        },
        10,
    };
}

#[test]
fn test_for() {
    assert_eq! {
        rune! { i64 =>
            use std::iter::range;

            pub fn main() {
                let a = 0;
                let it = range(0, 10);

                for v in it {
                    a = a + 1;
                }

                a
            }
        },
        10,
    };

    assert_eq! {
        rune! { i64 =>
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
        },
        10,
    };

    assert! {
        rune! { bool =>
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

                a is unit
            }
        },
    };
}

#[test]
fn test_return() {
    assert_eq! {
        rune! { i64 =>
            use std::iter::range;

            pub fn main() {
                for v in range(0, 20) {
                    if v == 10 {
                        return v;
                    }
                }

                0
            }
        },
        10,
    };
}

#[test]
fn test_is() {
    assert! {
        !rune! { bool =>
            pub fn main() {
                {} is Object
            }
        },
    };

    assert!(rune!(bool => pub fn main() { #{} is Object }));
    assert!(rune!(bool => pub fn main() { () is unit }));
    assert!(rune!(bool => fn foo() {} pub fn main() { foo() is unit }));
    assert!(rune!(bool => pub fn main() {{} is unit }));
    assert!(rune!(bool => pub fn main() { true is bool }));
    assert!(rune!(bool => pub fn main() { 'a' is char }));
    assert!(rune!(bool => pub fn main() { 42 is int }));
    assert!(rune!(bool => pub fn main() { 42.1 is float }));
    assert!(rune!(bool => pub fn main() { "hello" is String }));
    assert!(rune!(bool => pub fn main() { #{"hello": "world"} is Object }));
    assert!(rune!(bool => pub fn main() { ["hello", "world"] is Vec }));
}

#[test]
fn test_destructuring() {
    assert_eq! {
        rune! { i64 =>
            fn foo(n) {
                [n, n + 1]
            }

            pub fn main() {
                let [a, b] = foo(3);
                a + b
            }
        },
        7,
    };
}

#[test]
fn test_if_pattern() {
    assert! {
        rune! { bool =>
            pub fn main() {
                if let [value] = [()] {
                    true
                } else {
                    false
                }
            }
        },
    };

    assert! {
        !rune! { bool =>
            pub fn main() {
                if let [value] = [(), ()] {
                    true
                } else {
                    false
                }
            }
        },
    };

    assert_eq! {
        rune! { i64 =>
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
        },
        2,
    };
}

#[test]
fn test_break_label() {
    assert_eq! {
        rune! { i64 =>
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
        },
        77,
    };
}

#[test]
fn test_string_concat() {
    assert_eq! {
        rune! { String =>
            pub fn main() {
                let foo = String::from_str("foo");
                foo += "/bar" + "/baz";
                foo
            }
        },
        "foo/bar/baz",
    };
}

#[test]
fn test_template_string() {
    assert_eq! {
        rune_s! { String => r#"
            pub fn main() {
                let name = "John Doe";
                `Hello ${name}, I am ${1 - 10} years old!`
            }
        "#},
        "Hello John Doe, I am -9 years old!",
    };

    // Contrived expression with an arbitrary sub-expression.
    // This tests that the temporary variables used during calculations do not
    // accidentally clobber the scope.
    assert_eq! {
        rune_s! { String => r#"
            pub fn main() {
                let name = "John Doe";

                `Hello ${name}, I am ${{
                    let a = 20;
                    a += 2;
                    a
                }} years old!`
            }
        "#},
        "Hello John Doe, I am 22 years old!",
    };
}

#[test]
fn test_variants_as_functions() {
    assert_eq! {
        rune! { i64 =>
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
        },
        3,
    };
}

#[test]
fn test_iter_drop() {
    assert_eq! {
        rune! { i64 =>
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
        },
        24,
    };
}

#[test]
fn test_async_fn() {
    assert_eq! {
        rune! { i64 =>
            async fn foo(a, b) {
                b / a
            }

            fn bar(a, b) {
                b / a
            }

            pub async fn main() {
                foo(2, 4).await + bar(2, 8)
            }
        },
        6,
    };
}

#[test]
fn test_complex_field_access() {
    assert_eq! {
        rune_s! {
            Option<i64> => r#"
            fn foo() {
                #{hello: #{world: 42}}
            }

            pub fn main() {
                Some((foo()).hello["world"])
            }
            "#
        },
        Some(42),
    };
}

#[test]
fn test_index_get() {
    assert_eq! {
        rune! { i64 =>
            struct Named(a, b, c);
            enum Enum { Named(a, b, c) }

            fn a() { [1, 2, 3] }
            fn b() { (2, 3, 4) }
            fn c() { Named(3, 4, 5) }
            fn d() { Enum::Named(4, 5, 6) }

            pub fn main() {
                (a())[1] + (b())[1] + (c())[1] + (d())[1] + (a()).2 + (b()).2 + (c()).2 + (d()).2
            }
        },
        32,
    };
}
