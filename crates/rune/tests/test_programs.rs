use st::VmError::*;

async fn run_main<T>(source: &str) -> st::Result<T>
where
    T: st::FromValue,
{
    let unit = rune::compile(source)?;
    let mut vm = st::Vm::new();
    let context = st::Context::with_default_packages()?;
    let task: st::Task<T> = vm.call_function(&context, &unit, &["main"], ())?;
    let output = task.run_to_completion().await?;
    Ok(output)
}

/// Run the given program as a test.
macro_rules! test {
    ($ty:ty => $source:expr) => {
        run_main::<$ty>($source).await.unwrap()
    };
}

macro_rules! test_err {
    ($pat:pat => $cond:expr, $source:expr) => {{
        let e = run_main::<()>($source).await.unwrap_err();

        let e = match e.downcast_ref::<st::VmError>() {
            Some(e) => e,
            None => {
                panic!("{:?}", e);
            }
        };

        match e {
            $pat => ($cond),
            _ => {
                panic!("expected error `{}` but was `{:?}`", stringify!($pat), e);
            }
        }
    }};
}

#[tokio::test]
async fn test_small_programs() {
    assert_eq!(test!(u64 => r#"fn main() { 42 }"#), 42u64);
    assert_eq!(test!(() => r#"fn main() {}"#), ());

    assert_eq! {
        test!(i64 => r#"
        fn main() {
            let a = 1;
            let b = 2;
            let c = a + b;
            let d = c * 2;
            let e = d / 3;
            e
        }"#),
        2,
    };
}

#[tokio::test]
async fn test_if() {
    assert_eq! {
        test!(i64 => r#"
        fn main() {
            let n = 2;

            if n > 5 {
                10
            } else {
                0
            }
        }"#),
        0,
    };

    assert_eq! {
        test!(i64 => r#"
        fn main() {
            let n = 6;

            if n > 5 {
                10
            } else {
                0
            }
        }
        "#),
        10,
    };
}

#[tokio::test]
async fn test_block() {
    assert_eq! {
        test!(i64 => r#"
        fn main() {
            let b = 10;

            let n = {
                let a = 10;
                a + b
            };

            n + 1
        }"#),
        21,
    };
}

#[tokio::test]
async fn test_shadowing() {
    assert_eq! {
        test!(i64 => r#"
        fn main() {
            let a = 10;
            let a = a;
            a
        }"#),
        10,
    };
}

#[tokio::test]
async fn test_arrays() {
    assert_eq! {
        test!(() => "fn main() { let v = [1, 2, 3, 4, 5]; }"),
        (),
    };
}

#[tokio::test]
async fn test_while() {
    assert_eq! {
        test!(i64 => r#"
        fn main() {
            let a = 0;

            while a < 10 {
                a = a + 1;
            }

            a
        }"#),
        10,
    };

    assert_eq! {
        test!(i64 => r#"
        fn main() {
            let a = 0;

            let a = while a >= 0 {
                if a >= 10 {
                    break a;
                }

                a = a + 1;
            };

            a
        }"#),
        10,
    };
}

#[tokio::test]
async fn test_loop() {
    assert_eq! {
        test!(i64 => r#"
        fn main() {
            let a = 0;

            loop {
                if a >= 10 {
                    break;
                }

                a = a + 1;
            }

            a
        }"#),
        10,
    };

    assert_eq! {
        test!(i64 => r#"
        fn main() {
            let n = 0;

            let n = loop {
                if n >= 10 {
                    break n;
                }

                n = n + 1;
            };

            n
        }"#),
        10,
    };
}

#[tokio::test]
async fn test_for() {
    assert_eq! {
        test!(i64 => r#"
        use std::iter::range;

        fn main() {
            let a = 0;
            let it = range(0, 10);

            for v in it {
                a = a + 1;
            }

            a
        }"#),
        10,
    };

    assert_eq! {
        test!(i64 => r#"
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
        }"#),
        10,
    };
}

#[tokio::test]
async fn test_return() {
    assert_eq! {
        test!(i64 => r#"
        use std::iter::range;

        fn main() {
            for v in range(0, 20) {
                if v == 10 {
                    return v;
                }
            }

            0
        }"#),
        10,
    };
}

#[tokio::test]
async fn test_is() {
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
        test!(bool => r#"fn main() { ["hello", "world"] is Array }"#),
        true,
    };
}

#[tokio::test]
async fn test_match() {
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

#[tokio::test]
async fn test_array_match() {
    assert_eq! {
        test!(() => r#"fn main() { match [] { [..] => true } }"#),
        (),
    };

    assert_eq! {
        test!(bool => r#"fn main() { match [] { [..] => true, _ => false } }"#),
        true,
    };

    assert_eq! {
        test!(() => r#"fn main() { match [1, 2] { [a, b] => a + 1 == b } }"#),
        (),
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

#[tokio::test]
async fn test_object_match() {
    assert_eq! {
        test!(() => r#"fn main() { match #{} { #{..} => true } }"#),
        (),
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

#[tokio::test]
async fn test_bad_pattern() {
    // Attempting to assign to an unmatched pattern leads to a panic.
    test_err!(
        Panic { reason: st::Panic::UnmatchedPattern } => {},
        r#"
        fn main() {
            let [] = [1, 2, 3];
        }
        "#
    );
}

#[tokio::test]
async fn test_destructuring() {
    assert_eq! {
        test!(i64 => r#"
        fn foo(n) {
            [n, n + 1]
        }

        fn main() {
            let [a, b] = foo(3);
            a + b
        }"#),
        7,
    };
}
