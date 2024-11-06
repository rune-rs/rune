prelude!();

use core::ops::ControlFlow;

#[test]
fn custom_try() -> Result<()> {
    #[derive(Any)]
    struct CustomResult(bool);

    let mut module = Module::new();

    module.ty::<CustomResult>()?;

    module.associated_function(&Protocol::TRY, |r: CustomResult| {
        if r.0 {
            ControlFlow::Continue(42i64)
        } else {
            ControlFlow::Break(Err::<Value, _>(0i64))
        }
    })?;

    let n: u32 = rune_n! {
        mod module,
        (CustomResult(true),),
        pub fn main(r) { r? }
    };

    assert_eq!(n, 42);

    let result: Result<(), i64> = rune_n! {
        mod module,
        (CustomResult(false),),
        pub fn main(r) { r? }
    };

    assert_eq!(result, Err(0));
    Ok(())
}
