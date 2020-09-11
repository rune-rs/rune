use rune::{Options, Sources, Warnings};
use runestick::{Any, Context, Item, Module, Source, Value, Vm};
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
    let mut module = Module::new(Item::empty());
    module.ty(&["Foo"]).build::<Foo>().unwrap();
    module
        .inst_fn(runestick::ADD_ASSIGN, Foo::add_assign)
        .unwrap();

    let mut context = Context::with_default_modules().unwrap();
    context.install(&module).unwrap();

    let context = Arc::new(context);

    let mut sources = Sources::new();

    sources.insert_default(Source::new(
        "test",
        r#"
        fn main(number) {
            number += 1;
        }
        "#,
    ));

    let unit = rune::load_sources(
        &context,
        &Options::default(),
        &mut sources,
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
