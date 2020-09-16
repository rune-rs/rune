use rune::{Errors, Options, Sources, Warnings};
use runestick::{Any, Context, Item, Module, Source, Vm};
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

fn main() -> runestick::Result<()> {
    let mut module = Module::new(Item::new());
    module.ty::<Foo>()?;
    module.inst_fn(runestick::ADD_ASSIGN, Foo::add_assign)?;

    let mut context = Context::with_default_modules()?;
    context.install(&module)?;

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
    )?;

    let vm = Vm::new(context, Arc::new(unit));

    let mut foo = Foo::default();

    let output = vm.call(&["main"], (&mut foo,))?;
    println!("output: {:?}", output);
    println!("output: {:?}", foo);
    Ok(())
}
