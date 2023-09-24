prelude!();

use std::sync::Arc;

#[derive(Any, Debug, Default)]
struct Foo {
    #[rune(get, set, copy)]
    number: i64,
    #[rune(get, set)]
    string: String,
}

#[test]
fn test_getter_setter() -> Result<()> {
    let mut module = Module::new();
    module.ty::<Foo>()?;

    let mut context = Context::with_default_modules()?;
    context.install(module)?;

    let mut sources = sources! {
        entry => {
            pub fn main(foo) {
                foo.number = foo.number + 1;
                foo.string = format!("{} World", foo.string);
            }
        }
    };

    let unit = prepare(&mut sources).with_context(&context).build()?;

    let mut vm = Vm::new(Arc::new(context.runtime()?), Arc::new(unit));

    let mut foo = Foo {
        number: 42,
        string: String::from("Hello"),
    };

    let output = vm.call(["main"], (&mut foo,))?;

    assert_eq!(foo.number, 43);
    assert_eq!(foo.string, "Hello World");

    assert!(matches!(output, Value::EmptyTuple));
    Ok(())
}
