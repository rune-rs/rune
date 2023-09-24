prelude!();

#[test]
fn test_get_const() -> Result<()> {
    let context = Context::with_default_modules()?;

    let mut sources = sources! {
        entry => {
            pub const LEET = 1337;
        }
    };

    let unit = prepare(&mut sources).with_context(&context).build()?;

    assert_eq!(
        unit.constant(Hash::type_hash(["LEET"]))
            .expect("successful lookup")
            .try_clone()?
            .into_value()
            .expect("could not allocate value")
            .into_integer()
            .expect("the inner value"),
        1337
    );
    Ok(())
}

#[test]
fn test_get_const_re_export() -> Result<()> {
    let context = Context::with_default_modules()?;

    let mut sources = sources! {
        entry => {
            mod inner {
                pub const LEET = 1337;
            }

            pub use inner::LEET;
        },
    };

    let unit = prepare(&mut sources).with_context(&context).build()?;

    assert_eq!(
        unit.constant(Hash::type_hash(["LEET"]))
            .expect("successful lookup")
            .try_clone()?
            .into_value()
            .expect("could not allocate value")
            .into_integer()
            .expect("the inner value"),
        1337
    );
    Ok(())
}

#[test]
fn test_get_const_nested() -> Result<()> {
    let context = Context::with_default_modules()?;

    let mut sources = sources! {
        entry => {
            pub mod inner {
                pub const LEET = 1337;
            }
        },
    };

    let unit = prepare(&mut sources).with_context(&context).build()?;

    assert_eq!(
        unit.constant(Hash::type_hash(["inner", "LEET"]))
            .expect("successful lookup")
            .try_clone()?
            .into_value()
            .expect("could not allocate value")
            .into_integer()
            .expect("the inner value"),
        1337
    );
    Ok(())
}
