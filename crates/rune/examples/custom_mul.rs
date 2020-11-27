//! This example showcases overloading the multiplication protocol for a
//! specific type `Foo`.

use rune::{Errors, Options, Sources, Warnings};
use runestick::{Any, Context, FromValue, Module, Protocol, Source, Vm};
use std::sync::Arc;

#[derive(Debug, Default, Any)]
struct Foo {
    field: u32,
}

impl Foo {
    fn mul(self, b: u32) -> Self {
        Self {
            field: self.field * b,
        }
    }
}

fn main() -> runestick::Result<()> {
    let mut context = Context::with_default_modules()?;

    let mut module = Module::with_item(&["module"]);
    module.ty::<Foo>()?;
    module.inst_fn(Protocol::MUL, Foo::mul)?;
    context.install(&module)?;

    let mut sources = Sources::new();

    sources.insert(Source::new(
        "test",
        r#"
        pub fn main(foo) {
            foo * 5
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

    let vm = Vm::new(Arc::new(context), Arc::new(unit));
    let output = vm.execute(&["main"], (Foo { field: 5 },))?.complete()?;
    let output = Foo::from_value(output)?;

    println!("output: {:?}", output);
    Ok(())
}
