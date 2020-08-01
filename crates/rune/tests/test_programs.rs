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
async fn test_pointers() {
    assert_eq! {
        test!(i64 => r#"
        fn foo(n) {
            *n = 1;
        }

        fn main() {
            let n = 0;
            foo(&n);
            n
        }"#),
        1
    };

    assert_eq! {
        test!(i64 => r#"
        fn foo(n, u, v) {
            *n = *v;
        }

        fn main() {
            let n = 0;
            let u = 1;
            let v = 2;
            foo(&n, &u, &v);
            n
        }"#),
        2
    };

    test_err! {
        IllegalPtrReplace { target_ptr: 0, value_ptr: 2 } => (),
        r#"
        fn foo(n) {
            // trying to replace a n with a pointer that points to a later
            // stack location.
            let v = 5;
            *n = &v;
        }

        fn main() {
            let n = 0;
            foo(&n);
        }
        "#
    };
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
