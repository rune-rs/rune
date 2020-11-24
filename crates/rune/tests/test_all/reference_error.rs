use rune::{Errors, Options, Sources, Warnings};
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

    let mut module = Module::empty();
    module.function(&["take_it"], take_it).unwrap();

    let mut context = Context::with_default_modules().unwrap();
    context.install(&module).unwrap();

    let context = Arc::new(context);

    let mut sources = Sources::new();
    sources.insert(Source::new(
        "test",
        r#"fn main(number) { take_it(number) }"#,
    ));

    let mut errors = Errors::new();

    let unit = rune::load_sources(
        &context,
        &Options::default(),
        &mut sources,
        &mut errors,
        &mut Warnings::disabled(),
    )
    .unwrap();

    let vm = Vm::new(context, Arc::new(unit));

    let mut foo = Foo::default();
    assert_eq!(foo.value, 0);

    // This should error, because we're trying to acquire an `Ref` out of a
    // passed in reference.
    assert!(vm.call(&["main"], (&mut foo,)).is_err());
}
