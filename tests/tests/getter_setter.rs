use rune::{Diagnostics, Options, Sources};
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

    let mut diagnostics = Diagnostics::new();

    let unit = rune::load_sources(
        &context,
        &Options::default(),
        &mut sources,
        &mut diagnostics,
    )
    .unwrap();

    let vm = Vm::new(Arc::new(context.runtime()), Arc::new(unit));

    let mut test = Foo {
        number: 42,
        string: String::from("Hello"),
    };

    let output = vm.call(&["main"], (&mut test,)).unwrap();

    assert_eq!(test.number, 43);
    assert_eq!(test.string, "Hello World");

    assert!(matches!(output, Value::Unit));
}
