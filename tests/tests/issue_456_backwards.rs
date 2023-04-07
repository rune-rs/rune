use rune_tests::prelude::*;

#[test]
#[allow(deprecated)]
fn test_456_backwards() -> Result<()> {
    let mut context = Context::default();
    context.install(rune::modules::bytes::module()?)?;
    assert!(context.install(rune::modules::bytes::module()?).is_err());

    macro_rules! test_module {
        ($name:ident $(, $extra:expr)?) => {
            let mut context = Context::default();
            // NB: this should not cause an error, since they're running under the
            // special non-conflict mode.
            context.install(rune::modules::$name::module($($extra)*)?)?;
            assert!(!context.install(modules::$name::module(false)?).is_err());
        
            let mut context = Context::with_default_modules()?;
            // NB: this should not cause an error, since they're running under the
            // special non-conflict mode.
            assert!(!context.install(modules::$name::module(false)?).is_err());
        }
    }

    test_module!(core);
    test_module!(fmt);
    test_module!(io, false);
    test_module!(macros);
    test_module!(test);
    Ok(())
}
