use rune::{Options, Warnings};
use runestick::{Context, FromValue, Source, Vm};
use std::sync::Arc;

fn main() -> runestick::Result<()> {
    let context = Context::with_default_modules()?;

    let unit = rune::load_source(
        &context,
        &Options::default(),
        Source::new(
            "test",
            r#"
             fn main(number) {
                 number + 10
             }
             "#,
        ),
        &mut Warnings::disabled(),
    )?;

    let vm = Vm::new(Arc::new(context), Arc::new(unit));
    let output = vm.call(&["main"], (33i64,))?.complete()?;
    let output = i64::from_value(output)?;

    println!("output: {}", output);
    Ok(())
}
