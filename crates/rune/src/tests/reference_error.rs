prelude!();

use std::sync::Arc;

#[test]
fn test_reference_error() -> Result<()> {
    #[derive(Debug, Default, Any)]
    struct Foo {
        value: i64,
    }

    fn take_it(this: Shared<AnyObj>) -> VmResult<()> {
        // NB: this will error, since this is a reference.
        let _ = vm_try!(this.into_ref());
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
