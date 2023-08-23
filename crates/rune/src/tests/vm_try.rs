prelude!();

use core::ops::ControlFlow;

#[test]
fn test_unwrap() {
    let out: Result<i64, i64> = rune! {
        fn foo(a, b) {
            Ok(b / a)
        }

        fn bar(a, b) {
            Err(b / a)
        }

        pub fn main() {
            Ok(foo(2, 4)? + bar(3, 9)?)
        }
    };
    assert_eq!(out, Err(3));

    let out: Option<bool> = rune! {
        struct Bar {
            x,
            y,
        }

        pub fn main() {
            (Bar{ x: Some(1), y: None? }).x
        }
    };
    assert_eq!(out, None);

    let out: Result<i64, i64> = rune! {
        fn foo(a, b) {
            Ok(b / a)
        }

        pub fn main() {
            Ok(foo(2, 4)? + {
                Err(6 / 2)
            }?)
        }
    };
    assert_eq!(out, Err(3));
}

#[test]
fn custom_try() -> Result<()> {
    #[derive(Any)]
    struct CustomResult(bool);

    let mut module = Module::new();

    module.ty::<CustomResult>()?;

    module.associated_function(Protocol::TRY, |r: CustomResult| {
        if r.0 {
            ControlFlow::Continue(42i64)
        } else {
            ControlFlow::Break(Err::<Value, _>(0i64))
        }
    })?;

    assert_eq!(
        42,
        rune_n! {
            &module,
            (CustomResult(true),),
            i64 => pub fn main(r) { r? }
        }
    );

    assert_eq!(
        Err(0),
        rune_n! {
            &module,
            (CustomResult(false),),
            Result<(), i64> => pub fn main(r) { r? }
        }
    );

    Ok(())
}
