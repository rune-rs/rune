prelude!();

use core::ops::ControlFlow;

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
