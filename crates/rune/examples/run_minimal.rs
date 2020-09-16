use rune::{Errors, Options, Sources, Warnings};
use runestick::{Context, FromValue, Module, Source};
use std::sync::Arc;

fn main() -> runestick::Result<()> {
    let mut context = Context::default();

    let mut module = Module::default();
    module.function(&["add"], |a: i64| a + 1)?;
    context.install(&module)?;

    let mut sources = Sources::new();
    sources.insert(Source::new("test", r#"fn main(a) { add(a) }"#));

    let mut errors = Errors::new();

    let unit = rune::load_sources(
        &context,
        &Options::default(),
        &mut sources,
        &mut errors,
        &mut Warnings::disabled(),
    )?;

    let vm = runestick::Vm::new(Arc::new(context), Arc::new(unit));
    let output = i64::from_value(vm.execute(&["main"], (1,))?.complete()?)?;

    println!("output: {}", output);
    Ok(())
}
