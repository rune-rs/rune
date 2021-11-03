use rune::{Diagnostics, Options, Sources};
use runestick::{Context, FromValue, Module, Source, Vm};
use std::sync::Arc;

fn divide_by_three(value: i64) -> i64 {
    value / 3
}

#[tokio::main]
async fn main() -> runestick::Result<()> {
    let mut my_module = Module::with_item(&["mymodule"]);
    my_module.inst_fn("divide_by_three", divide_by_three)?;

    let mut context = Context::with_default_modules()?;
    context.install(&my_module)?;

    let options = Options::default();

    let mut sources = Sources::new();
    sources.insert(Source::new(
        "test",
        r#"
        pub fn main(number) {
            number.divide_by_three()
        }
        "#,
    ));

    let mut diagnostics = Diagnostics::new();

    let unit = rune::load_sources(&context, &options, &mut sources, &mut diagnostics)?;

    let mut vm = Vm::new(Arc::new(context.runtime()), Arc::new(unit));
    let output = vm.execute(&["main"], (33i64,))?.complete()?;
    let output = i64::from_value(output)?;

    println!("output: {}", output);
    Ok(())
}
