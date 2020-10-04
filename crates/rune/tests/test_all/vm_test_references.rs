use rune::{Errors, Options, Sources, Warnings};
use runestick::{Any, AnyObj, Context, Module, Shared, Source, Value, Vm, VmError};
use std::sync::Arc;

#[derive(Debug, Default, Any)]
struct Foo {
    value: i64,
}

impl Foo {
    fn add_assign(&mut self, value: i64) {
        self.value += value;
    }
}

#[test]
fn vm_test_references() {
    let mut module = Module::empty();
    module.ty::<Foo>().unwrap();
    module
        .inst_fn(runestick::ADD_ASSIGN, Foo::add_assign)
        .unwrap();

    let mut context = Context::with_default_modules().unwrap();
    context.install(&module).unwrap();

    let context = Arc::new(context);

    let mut sources = Sources::new();
    sources.insert(Source::new(
        "test",
        r#"
        fn main(number) {
            number += 1;
        }
        "#,
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
    let output = vm.call(&["main"], (&mut foo,)).unwrap();
    assert_eq!(foo.value, 1);
    assert!(matches!(output, Value::Unit));
}

#[test]
fn vm_test_references_error() {
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
