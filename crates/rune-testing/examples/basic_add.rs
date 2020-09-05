use runestick::{Context, FromValue, Source};
use std::sync::Arc;

fn main() -> runestick::Result<()> {
    let context = Context::with_default_modules()?;

    let options = rune::Options::default();
    let mut warnings = rune::Warnings::default();

    let unit = rune::load_source(
        &context,
        &options,
        &mut warnings,
        Source::new(
            "test",
            r#"
             fn main(number) {
                 number + 10
             }
             "#,
        ),
    )?;

    let vm = runestick::Vm::new(Arc::new(context), Arc::new(unit));
    let output = vm.call(&["main"], (33i64,))?.complete()?;
    let output = i64::from_value(output)?;

    println!("output: {}", output);
    Ok(())
}
