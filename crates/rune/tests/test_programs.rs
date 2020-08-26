use futures_executor::block_on;
use runestick::VmError::*;

async fn run_main<T>(source: &str) -> Result<T, Box<dyn std::error::Error>>
where
    T: runestick::FromValue,
{
    let context = runestick::Context::with_default_packages()?;
    let (unit, _) = rune::compile(&context, source)?;
    let mut vm = runestick::Vm::new();
    let context = runestick::Context::with_default_packages()?;
    let mut task: runestick::Task<T> = vm.call_function(&unit, &context, &["main"], ())?;
    let output = task.run_to_completion().await?;
    Ok(output)
}

/// Run the given program as a test.
macro_rules! test {
    ($ty:ty => $source:expr) => {
        block_on(run_main::<$ty>($source)).expect("program to run successfully")
    };
}

macro_rules! test_vm_error {
    ($source:expr, $pat:pat => $cond:expr) => {{
        let e = block_on(run_main::<()>($source)).unwrap_err();

        let e = match e.downcast_ref::<runestick::VmError>() {
            Some(e) => e,
            None => {
                panic!("{:?}", e);
            }
        };

        match e {
            $pat => $cond,
            _ => {
                panic!("expected error `{}` but was `{:?}`", stringify!($pat), e);
            }
        }
    }};
}

#[test]
fn test_small_programs() {
    assert_eq!(test!(u64 => r#"fn main() { 42 }"#), 42u64);
    assert_eq!(test!(() => r#"fn main() {}"#), ());

    assert_eq! {
        test! {
            i64 => r#"
            fn main() {
                let a = 1;
                let b = 2;
                let c = a + b;
                let d = c * 2;
                let e = d / 3;
                e
            }
            "#
        },
        2,
    };
}

#[test]
fn test_boolean_ops() {
    assert_eq! {
        test!(bool => r#"fn main() { true && true }"#),
        true,
    };

    assert_eq! {
        test!(bool => r#"fn main() { true && false }"#),
        false,
    };

    assert_eq! {
        test!(bool => r#"fn main() { false && true }"#),
        false,
    };

    assert_eq! {
        test!(bool => r#"fn main() { false && false }"#),
        false,
    };

    assert_eq! {
        test!(bool => r#"fn main() { true || true }"#),
        true,
    };

    assert_eq! {
        test!(bool => r#"fn main() { true || false }"#),
        true,
    };

    assert_eq! {
        test!(bool => r#"fn main() { false || true }"#),
        true,
    };

    assert_eq! {
        test!(bool => r#"fn main() { false || false }"#),
        false,
    };
}

#[test]
fn test_if() {
    assert_eq! {
        test! {
            i64 => r#"
            fn main() {
                let n = 2;

                if n > 5 {
                    10
                } else {
                    0
                }
            }
            "#
        },
        0,
    };

    assert_eq! {
        test!{
            i64 => r#"
            fn main() {
                let n = 6;

                if n > 5 {
                    10
                } else {
                    0
                }
            }
            "#
        },
        10,
    };
}

#[test]
fn test_block() {
    assert_eq! {
        test! {
            i64 => r#"
            fn main() {
                let b = 10;

                let n = {
                    let a = 10;
                    a + b
                };

                n + 1
            }
            "#
        },
        21,
    };
}

#[test]
fn test_shadowing() {
    assert_eq! {
        test! {
            i64 => r#"
            fn main() {
                let a = 10;
                let a = a;
                a
            }
            "#
        },
        10,
    };
}

#[test]
fn test_vectors() {
    assert_eq! {
        test!(() => "fn main() { let v = [1, 2, 3, 4, 5]; }"),
        (),
    };
}

#[test]
fn test_while() {
    assert_eq! {
        test!{
            i64 => r#"
            fn main() {
                let a = 0;

                while a < 10 {
                    a = a + 1;
                }

                a
            }
            "#
        },
        10,
    };

    assert_eq! {
        test! {
            i64 => r#"
            fn main() {
                let a = 0;

                let a = while a >= 0 {
                    if a >= 10 {
                        break a;
                    }

                    a = a + 1;
                };

                a
            }
            "#
        },
        10,
    };
}

#[test]
fn test_loop() {
    assert_eq! {
        test! {
            runestick::VecTuple<(i64, bool)> => r#"
            fn main() {
                let a = 0;

                let value = loop {
                    if a >= 10 {
                        break;
                    }

                    a = a + 1;
                };

                [a, value is unit]
            }
            "#
        },
        runestick::VecTuple((10, true)),
    };

    assert_eq! {
        test! {
            i64 => r#"
            fn main() {
                let n = 0;

                let n = loop {
                    if n >= 10 {
                        break n;
                    }

                    n = n + 1;
                };

                n
            }
            "#
        },
        10,
    };
}

#[test]
fn test_for() {
    assert_eq! {
        test! {
            i64 => r#"
            use std::iter::range;

            fn main() {
                let a = 0;
                let it = range(0, 10);

                for v in it {
                    a = a + 1;
                }

                a
            }
            "#
        },
        10,
    };

    assert_eq! {
        test! {
            i64 => r#"
            use std::iter::range;

            fn main() {
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
            "#
        },
        10,
    };

    assert_eq! {
        test! {
            bool => r#"
            use std::iter::range;

            fn main() {
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
            "#
        },
        true,
    };
}

#[test]
fn test_return() {
    assert_eq! {
        test! {
            i64 => r#"
            use std::iter::range;

            fn main() {
                for v in range(0, 20) {
                    if v == 10 {
                        return v;
                    }
                }

                0
            }
            "#
        },
        10,
    };
}

#[test]
fn test_is() {
    assert_eq! {
        test!(bool => r#"
        fn main() {
            {} is Object
        }"#),
        false,
    };

    assert_eq! {
        test!(bool => r#"fn main() { #{} is Object }"#),
        true,
    };

    assert_eq! {
        test!(bool => r#"fn main() { () is unit }"#),
        true,
    };

    assert_eq! {
        test!(bool => r#"fn foo() {} fn main() { foo() is unit }"#),
        true,
    };

    assert_eq! {
        test!(bool => r#"fn main() { {} is unit }"#),
        true,
    };

    assert_eq! {
        test!(bool => r#"fn main() { true is bool }"#),
        true,
    };
    assert_eq! {
        test!(bool => r#"fn main() { 'a' is char }"#),
        true,
    };
    assert_eq! {
        test!(bool => r#"fn main() { 42 is int }"#),
        true,
    };
    assert_eq! {
        test!(bool => r#"fn main() { 42.1 is float }"#),
        true,
    };
    assert_eq! {
        test!(bool => r#"fn main() { "hello" is String }"#),
        true,
    };
    assert_eq! {
        test!(bool => r#"fn main() { #{"hello": "world"} is Object }"#),
        true,
    };
    assert_eq! {
        test!(bool => r#"fn main() { ["hello", "world"] is Vec }"#),
        true,
    };
}

#[test]
fn test_match() {
    assert_eq! {
        test!(i64 => r#"fn main() { match 1 { _ => 10 } }"#),
        10,
    };

    assert_eq! {
        test!(i64 => r#"fn main() { match 10 { n => 10 } }"#),
        10,
    };

    assert_eq! {
        test!(char => r#"fn main() { match 'a' { 'a' => 'b', n => n } }"#),
        'b',
    };

    assert_eq! {
        test!(i64 => r#"fn main() { match 10 { n => n } }"#),
        10,
    };

    assert_eq! {
        test!(i64 => r#"fn main() { match 10 { 10 => 5, n => n } }"#),
        5,
    };

    assert_eq! {
        test!(String => r#"fn main() { match "hello world" { "hello world" => "hello john", n => n } }"#),
        "hello john",
    };
}

#[test]
fn test_vec_match() {
    assert_eq! {
        test!(bool => r#"fn main() { match [] { [..] => true } }"#),
        true,
    };

    assert_eq! {
        test!(bool => r#"fn main() { match [] { [..] => true, _ => false } }"#),
        true,
    };

    assert_eq! {
        test!(bool => r#"fn main() { match [1, 2] { [a, b] => a + 1 == b } }"#),
        true,
    };

    assert_eq! {
        test!(() => r#"fn main() { match [] { [a, b] => a + 1 == b } }"#),
        (),
    };

    assert_eq! {
        test!(bool => r#"fn main() { match [1, 2] { [a, b] => a + 1 == b, _ => false } }"#),
        true,
    };

    assert_eq! {
        test!(bool => r#"fn main() { match [1, 2] { [a, b, ..] => a + 1 == b, _ => false } }"#),
        true,
    };

    assert_eq! {
        test!(bool => r#"fn main() { match [1, 2] { [1, ..] => true, _ => false } }"#),
        true,
    };

    assert_eq! {
        test!(bool => r#"fn main() { match [1, 2] { [] => true, _ => false } }"#),
        false,
    };

    assert_eq! {
        test!(bool => r#"fn main() { match [1, 2] { [1, 2] => true, _ => false } }"#),
        true,
    };

    assert_eq! {
        test!(bool => r#"fn main() { match [1, 2] { [1] => true, _ => false } }"#),
        false,
    };

    assert_eq! {
        test!(bool => r#"fn main() { match [1, [2, 3]] { [1, [2, ..]] => true, _ => false } }"#),
        true,
    };

    assert_eq! {
        test!(bool => r#"fn main() { match [1, []] { [1, [2, ..]] => true, _ => false } }"#),
        false,
    };

    assert_eq! {
        test!(bool => r#"fn main() { match [1, [2, 3]] { [1, [2, 3]] => true, _ => false } }"#),
        true,
    };

    assert_eq! {
        test!(bool => r#"fn main() { match [1, [2, 4]] { [1, [2, 3]] => true, _ => false } }"#),
        false,
    };
}

#[test]
fn test_object_match() {
    assert_eq! {
        test!(bool => r#"fn main() { match #{} { #{..} => true } }"#),
        true,
    };

    assert_eq! {
        test!(bool => r#"fn main() { match #{foo: true} { #{foo} => foo, _ => false } }"#),
        true,
    };

    assert_eq! {
        test!(bool => r#"fn main() { match #{} { #{..} => true, _ => false } }"#),
        true,
    };

    assert_eq! {
        test!(bool => r#"fn main() { match #{"foo": 10, "bar": 0} { #{"foo": v, ..} => v == 10, _ => false } }"#),
        true,
    };

    assert_eq! {
        test!(bool => r#"fn main() { match #{"foo": 10, "bar": 0} { #{"foo": v} => v == 10, _ => false } }"#),
        false,
    };

    assert_eq! {
        test!(bool => r#"fn main() { match #{"foo": 10, "bar": #{"baz": [1, 2]}} { #{"foo": v} => v == 10, _ => false } }"#),
        false,
    };

    assert_eq! {
        test!(bool => r#"fn main() { match #{"foo": 10, "bar": #{"baz": [1, 2]}} { #{"foo": v, ..} => v == 10, _ => false } }"#),
        true,
    };
}

#[test]
fn test_bad_pattern() {
    // Attempting to assign to an unmatched pattern leads to a panic.
    test_vm_error!(
        r#"
        fn main() {
            let [] = [1, 2, 3];
        }
        "#,
        Panic { reason } => {
            assert_eq!(reason.to_string(), "pattern did not match");
        }
    );
}

#[test]
fn test_destructuring() {
    assert_eq! {
        test! {
            i64 => r#"
            fn foo(n) {
                [n, n + 1]
            }

            fn main() {
                let [a, b] = foo(3);
                a + b
            }
            "#
        },
        7,
    };
}

#[test]
fn test_if_pattern() {
    assert_eq! {
        test! {
            bool => r#"
            fn main() {
                if let [value] = [()] {
                    true
                } else {
                    false
                }
            }
            "#
        },
        true,
    };

    assert_eq! {
        test! {
            bool => r#"
            fn main() {
                if let [value] = [(), ()] {
                    true
                } else {
                    false
                }
            }
            "#
        },
        false,
    };

    assert_eq! {
        test! {
            i64 => r#"
            fn main() {
                let value = [(), (), 2];

                if let [(), ()] = value {
                    1
                } else if let [(), (), c] = value {
                    c
                } else {
                    3
                }
            }
            "#
        },
        2,
    };
}

#[test]
fn test_break_label() {
    assert_eq! {
        test! {
            i64 => r#"
            use std::iter::range;

            fn main() {
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
            "#
        },
        77,
    };
}

#[test]
fn test_literal() {
    assert_eq! {
        test!(char => r#"fn main() { '\u{1F4AF}' }"#),
        '💯',
    };
}

#[test]
fn test_string_concat() {
    assert_eq! {
        test! {
            String => r#"
            fn main() {
                let foo = String::from_str("foo");
                foo += "/bar" + "/baz";
                foo
            }
            "#
        },
        "foo/bar/baz",
    };
}

#[test]
fn test_template_string() {
    assert_eq! {
        test! {
            String => r#"
            fn main() {
                let name = "John Doe";
                `Hello {name}, I am {1 - 10} years old!`
            }
            "#
        },
        "Hello John Doe, I am -9 years old!",
    };

    // Contrived expression with an arbitrary sub-expression.
    // This tests that the temporary variables used during calculations do not
    // accidentally clobber the scope.
    assert_eq! {
        test! {
            String => r#"
            fn main() {
                let name = "John Doe";

                `Hello {name}, I am {{
                    let a = 20;
                    a += 2;
                    a
                }} years old!`
            }
            "#
        },
        "Hello John Doe, I am 22 years old!",
    };
}

#[test]
fn test_match_custom_tuple() {
    assert_eq! {
        test! {
            i64 => r#"
            fn main() { match Err("err") { Err("err") => 1,  _ => 2 } }
            "#
        },
        1,
    };

    assert_eq! {
        test! {
            i64 => r#"
            fn main() { match Err("err") { Ok("ok") => 1,  _ => 2 } }
            "#
        },
        2,
    };

    assert_eq! {
        test! {
            i64 => r#"
            fn main() { match Ok("ok") { Ok("ok") => 1,  _ => 2 } }
            "#
        },
        1,
    };

    assert_eq! {
        test! {
            i64 => r#"
            fn main() { match Some("value") { Some("value") => 1,  _ => 2 } }
            "#
        },
        1,
    };

    assert_eq! {
        test! {
            i64 => r#"
            fn main() { match Some("value") { None => 1,  _ => 2 } }
            "#
        },
        2,
    };

    assert_eq! {
        test! {
            i64 => r#"
            fn main() { match None { None => 1,  _ => 2 } }
            "#
        },
        1,
    };

    assert_eq! {
        test! {
            i64 => r#"
            fn main() { match None { None => 1,  _ => 2 } }
            "#
        },
        1,
    };

    assert_eq! {
        test! {
            i64 => r#"
            fn main() { match Option::None { None => 1,  _ => 2 } }
            "#
        },
        1,
    };

    assert_eq! {
        test! {
            bool => r#"
            fn main() {
                if let Some(a) = Some("hello") { true } else { false }
            }
            "#
        },
        true,
    };

    assert_eq! {
        test! {
            bool => r#"
            fn main() {
                if let Some(a) = None { true } else { false }
            }
            "#
        },
        false,
    };
}

#[test]
fn test_defined_tuple() {
    assert_eq! {
        test! {
            i64 => r#"
            struct MyType(a, b);

            fn main() { match MyType(1, 2) { MyType(a, b) => a + b,  _ => 0 } }
            "#
        },
        3,
    };

    assert_eq! {
        test! {
            i64 => r#"
            enum MyType { A(a, b), C(c), }

            fn main() { match MyType::A(1, 2) { MyType::A(a, b) => a + b,  _ => 0 } }
            "#
        },
        3,
    };

    assert_eq! {
        test! {
            i64 => r#"
            enum MyType { A(a, b), C(c), }

            fn main() { match MyType::C(4) { MyType::A(a, b) => a + b,  _ => 0 } }
            "#
        },
        0,
    };

    assert_eq! {
        test! {
            i64 => r#"
            enum MyType { A(a, b), C(c), }

            fn main() { match MyType::C(4) { MyType::C(a) => a,  _ => 0 } }
            "#
        },
        4,
    };
}

#[test]
fn test_variants_as_functions() {
    assert_eq! {
        test! {
            i64 => r#"
            enum Foo { A(a), B(b, c) }

            fn construct_tuple(tuple) {
                tuple(1, 2)
            }

            fn main() {
                let foo = construct_tuple(Foo::B);

                match foo {
                    Foo::B(a, b) => a + b,
                    _ => 0,
                }
            }
            "#
        },
        3,
    };
}

#[test]
fn test_struct_matching() {
    assert_eq! {
        test! {
            i64 => r#"
            struct Foo { a, b }

            fn main() {
                let foo = Foo {
                    a: 1,
                    b: 2,
                };

                match foo {
                    Foo { a, b } => a + b,
                    _ => 0,
                }
            }
            "#
        },
        3,
    };

    assert_eq! {
        test! {
            i64 => r#"
            struct Foo { a, b }

            fn main() {
                let b = 2;

                let foo = Foo {
                    a: 1,
                    b,
                };

                match foo {
                    Foo { a, b } => a + b,
                    _ => 0,
                }
            }
            "#
        },
        3,
    };
}

#[test]
fn test_iter_drop() {
    assert_eq! {
        test! {
            i64 => r#"
            fn main() {
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
            "#
        },
        24,
    };
}
