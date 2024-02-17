prelude!();

use std::sync::Arc;

#[test]
fn test_reference_error() -> Result<()> {
    #[derive(Debug, Default, Any)]
    struct Foo {
        value: i64,
    }

    // NB: Calling this should error, since it's a mutable reference.
    fn take_it(_: Ref<Foo>) -> VmResult<()> {
        VmResult::Ok(())
    }

    let mut module = Module::new();
    module.function("take_it", take_it).build()?;

    let mut context = Context::new();
    context.install(module)?;

    let mut sources = sources! {
        entry => {
            fn main(number) { take_it(number) }
        }
    };

    let unit = prepare(&mut sources).with_context(&context).build()?;

    let mut vm = Vm::new(Arc::new(context.runtime()?), Arc::new(unit));

    let mut foo = Foo::default();
    assert_eq!(foo.value, 0);

    // This should error, because we're trying to acquire an `Ref` out of a
    // passed in reference.
    assert!(vm.call(["main"], (&mut foo,)).is_err());
    Ok(())
}
