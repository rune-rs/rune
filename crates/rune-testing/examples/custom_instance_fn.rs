use runestick::{Context, FromValue, Hash, Item, Module, Source};
use std::sync::Arc;

fn divide_by_three(value: i64) -> i64 {
    value / 3
}

#[tokio::main]
async fn main() -> runestick::Result<()> {
    let mut my_module = Module::new(&["mymodule"]);
    my_module.inst_fn("divide_by_three", divide_by_three)?;

    let mut context = Context::with_default_modules()?;
    context.install(&my_module)?;

    let options = rune::Options::default();
    let mut warnings = rune::Warnings::default();

    let result = rune::load_source(
        &context,
        &options,
        &mut warnings,
        true,
        Source::new(
            "test",
            r#"
            fn call_instance_fn(number) {
                number.divide_by_three()
            }
            "#,
        ),
    );

    let unit = match result {
        Ok(unit) => unit,
        Err(error) => {
            use rune::termcolor;
            let mut writer = termcolor::StandardStream::stderr(termcolor::ColorChoice::Never);
            error.emit_diagnostics(&mut writer)?;
            return Ok(());
        }
    };

    let vm = runestick::Vm::new(Arc::new(context), Arc::new(unit));

    let output = vm
        .call_function(Hash::type_hash(Item::of(&["call_instance_fn"])), (33i64,))?
        .async_complete()
        .await?;

    let output = i64::from_value(output)?;
    println!("output: {}", output);
    Ok(())
}
