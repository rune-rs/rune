use rune::Hash;

#[test]
fn test_get_const() -> rune::Result<()> {
    let context = rune_modules::default_context()?;

    let mut sources = rune::sources! {
        entry => {
        pub const LOAD_COUNT = 1337;
        }
    };

    let unit = rune::prepare(&mut sources).with_context(&context).build()?;

    assert_eq!(
        unit.constant(Hash::constant("LOAD_COUNT"))
            .expect("successful lookup")
            .clone()
            .into_value()
            .into_integer()
            .expect("the inner value"),
        1337
    );
    Ok(())
}
