use rune::{Diagnostics, Options, Sources};
use runestick::{Any, Context, Module, Protocol, Source, Vm};
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
    let mut module = Module::new();
    module.ty::<Foo>()?;
    module.inst_fn(Protocol::ADD_ASSIGN, Foo::add_assign)?;

    let mut context = Context::with_default_modules()?;
    context.install(&module)?;

    let mut sources = Sources::new();
    sources.insert(Source::new(
        "test",
        r#"
        pub fn main(number) {
            number += 1;
        }
        "#,
    ));

    let mut diagnostics = Diagnostics::new();

    let unit = rune::load_sources(
        &context,
        &Options::default(),
        &mut sources,
        &mut diagnostics,
    )?;

    let vm = Vm::new(Arc::new(context.runtime()), Arc::new(unit));

    let mut test = Foo::default();

    let output = vm.call(&["main"], (&mut test,))?;
    println!("output: {:?}", output);
    println!("output: {:?}", test);
    Ok(())
}
