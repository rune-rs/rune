use rune::{Errors, Options, Sources, Warnings};
use runestick::{Any, Context, Module, Source, Value, Vm};
use std::sync::Arc;

#[derive(Any, Debug, Default)]
struct Foo {
    #[rune(get, set, copy)]
    number: i64,
    #[rune(get, set)]
    string: String,
}

#[test]
fn test_getter_setter() {
    let mut module = Module::new();
    module.ty::<Foo>().unwrap();

    let mut context = Context::with_default_modules().unwrap();
    context.install(&module).unwrap();

    let context = Arc::new(context);

    let mut sources = Sources::new();
    sources.insert(Source::new(
        "test",
        r#"
        pub fn main(foo) {
            foo.number = foo.number + 1;
            foo.string = `${foo.string} World`;
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

    let mut foo = Foo {
        number: 42,
        string: String::from("Hello"),
    };

    let output = vm.call(&["main"], (&mut foo,)).unwrap();

    assert_eq!(foo.number, 43);
    assert_eq!(foo.string, "Hello World");

    assert!(matches!(output, Value::Unit));
}
