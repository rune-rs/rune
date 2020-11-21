//! This example showcases overloading the multiplication protocol for a
//! specific type `Foo`.

use rune::{Errors, Options, Sources, Warnings};
use runestick::{Context, FromValue, Source, Vm};
use std::sync::Arc;

#[derive(Debug, Default, runestick::Any)]
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

    let mut module = runestick::Module::new(&["module"]);
    module.ty::<Foo>()?;
    module.inst_fn(runestick::MUL, Foo::mul)?;
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
