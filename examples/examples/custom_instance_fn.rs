use rune::{
    termcolor::{ColorChoice, StandardStream},
    Context, Diagnostics, FromValue, Module, Source, Sources, Vm,
};
use std::sync::Arc;

fn divide_by_three(value: i64) -> i64 {
    value / 3
}

#[tokio::main]
async fn main() -> rune::Result<()> {
    let mut my_module = Module::with_item(&["mymodule"]);
    my_module.inst_fn("divide_by_three", divide_by_three)?;

    let mut context = Context::with_default_modules()?;
    context.install(&my_module)?;
    let runtime = Arc::new(context.runtime());

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

    let result = rune::prepare(&context, &mut sources)
        .with_diagnostics(&mut diagnostics)
        .build();

    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        diagnostics.emit(&mut writer, &sources)?;
    }

    let unit = result?;

    let mut vm = Vm::new(runtime, Arc::new(unit));
    let output = vm.execute(&["main"], (33i64,))?.complete()?;
    let output = i64::from_value(output)?;

    println!("output: {}", output);
    Ok(())
}
