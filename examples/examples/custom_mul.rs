//! This example showcases overloading the multiplication protocol for a
//! specific type `Foo`.

use rune::{Any, Context, Diagnostics, FromValue, Module, Protocol, Source, Sources, Vm};
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

fn main() -> rune::Result<()> {
    let mut context = Context::with_default_modules()?;

    let mut module = Module::with_item(&["module"]);
    module.ty::<Foo>()?;
    module.inst_fn(Protocol::MUL, Foo::mul)?;
    context.install(&module)?;

    let runtime = Arc::new(context.runtime());

    let mut sources = Sources::new();
    sources.insert(Source::new(
        "test",
        r#"
        pub fn main(foo) {
            foo * 5
        }
        "#,
    ));

    let mut diagnostics = Diagnostics::new();

    let unit = rune::prepare(&context, &mut sources)
        .with_diagnostics(&mut diagnostics)
        .build()?;

    let mut vm = Vm::new(runtime, Arc::new(unit));
    let output = vm.call(&["main"], (Foo { field: 5 },))?;
    let output = Foo::from_value(output)?;

    println!("output: {:?}", output);
    Ok(())
}
