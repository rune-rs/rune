use rune::{Diagnostics, Options, Sources};
use runestick::{Any, AnyObj, Context, Module, Shared, Source, Vm, VmError};
use std::sync::Arc;

#[test]
fn test_reference_error() {
    #[derive(Debug, Default, Any)]
    struct Foo {
        value: i64,
    }

    fn take_it(this: Shared<AnyObj>) -> Result<(), VmError> {
        // NB: this will error, since this is a reference.
        let _ = this.into_ref()?;
        Ok(())
    }

    let mut module = Module::new();
    module.function(&["take_it"], take_it).unwrap();

    let mut context = Context::with_default_modules().unwrap();
    context.install(&module).unwrap();

    let mut sources = Sources::new();
    sources.insert(Source::new(
        "test",
        r#"fn main(number) { take_it(number) }"#,
    ));

    let mut diagnostics = Diagnostics::new();

    let unit = rune::load_sources(
        &context,
        &Options::default(),
        &mut sources,
        &mut diagnostics,
    )
    .unwrap();

    let vm = Vm::new(Arc::new(context.runtime()), Arc::new(unit));

    let mut test = Foo::default();
    assert_eq!(test.value, 0);

    // This should error, because we're trying to acquire an `Ref` out of a
    // passed in reference.
    assert!(vm.call(&["main"], (&mut test,)).is_err());
}
