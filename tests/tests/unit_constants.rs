use rune::Hash;

#[test]
fn test_get_const() -> rune::Result<()> {
    let context = rune_modules::default_context()?;

    let mut sources = rune::sources! {
        entry => {
            pub const LEET = 1337;
        }
    };

    let unit = rune::prepare(&mut sources).with_context(&context).build()?;

    assert_eq!(
        unit.constant(Hash::type_hash(["LEET"]))
            .expect("successful lookup")
            .clone()
            .into_value()
            .into_integer()
            .expect("the inner value"),
        1337
    );
    Ok(())
}

#[test]
fn test_get_const_re_export() -> rune::Result<()> {
    let context = rune_modules::default_context()?;

    let mut sources = rune::sources! {
        entry => {
            mod inner {
                pub const LEET = 1337;
            }

            pub use inner::LEET;
        },
    };

    let unit = rune::prepare(&mut sources).with_context(&context).build()?;

    assert_eq!(
        unit.constant(Hash::type_hash(["LEET"]))
            .expect("successful lookup")
            .clone()
            .into_value()
            .into_integer()
            .expect("the inner value"),
        1337
    );
    Ok(())
}

#[test]
fn test_get_const_nested() -> rune::Result<()> {
    let context = rune_modules::default_context()?;

    let mut sources = rune::sources! {
        entry => {
            pub mod inner {
                pub const LEET = 1337;
            }
        },
    };

    let unit = rune::prepare(&mut sources).with_context(&context).build()?;

    assert_eq!(
        unit.constant(Hash::type_hash(["inner", "LEET"]))
            .expect("successful lookup")
            .clone()
            .into_value()
            .into_integer()
            .expect("the inner value"),
        1337
    );
    Ok(())
}
