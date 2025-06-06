prelude!();

use crate::alloc::String;

#[derive(Any, Debug)]
struct Foo {
    #[rune(get, set, copy)]
    number: i64,
    #[rune(get, set)]
    string: String,
    #[rune(get, set)]
    shared_string: Shared<String>,
}

#[test]
fn test_getter_setter() -> Result<()> {
    let mut module = Module::new();
    module.ty::<Foo>()?;

    let mut context = Context::with_default_modules()?;
    context.install(module)?;
    let runtime = Arc::try_new(context.runtime()?)?;

    let mut sources = sources! {
        entry => {
            pub fn main(foo) {
                foo.number = foo.number + 1;
                foo.string = format!("{} World", foo.string);
                foo.shared_string = format!("{} Shared World", foo.shared_string);
                foo.shared_string
            }
        }
    };

    let unit = prepare(&mut sources).with_context(&context).build()?;
    let unit = Arc::try_new(unit)?;
    let mut vm = Vm::new(runtime, unit);

    let mut foo = Foo {
        number: 42,
        string: String::try_from("Hello")?,
        shared_string: Shared::new(String::try_from("Hello")?)?,
    };

    let output = vm.call(["main"], (&mut foo,))?;

    assert_eq!(foo.number, 43);
    assert_eq!(foo.string, "Hello World");
    assert_eq!(&*foo.shared_string.borrow_ref()?, "Hello Shared World");

    let string = output.downcast::<String>().unwrap();
    assert_eq!(string, "Hello Shared World");
    Ok(())
}
