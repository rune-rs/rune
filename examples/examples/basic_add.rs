use rune::{Diagnostics, Options, Sources};
use runestick::{Context, FromValue, Source, Vm};
use std::sync::Arc;

fn main() -> runestick::Result<()> {
    let context = Context::with_default_modules()?;
    let mut sources = Sources::new();

    sources.insert(Source::new(
        "test",
        r#"
        pub fn main(number) {
            number + 10
        }
        "#,
    ));

    let mut diagnostics = Diagnostics::without_warnings();

    let unit = rune::load_sources(
        &context,
        &Options::default(),
        &mut sources,
        &mut diagnostics,
    )?;

    let vm = Vm::new(Arc::new(context.runtime()), Arc::new(unit));
    let output = vm.execute(&["main"], (33i64,))?.complete()?;
    let output = i64::from_value(output)?;

    println!("output: {}", output);
    Ok(())
}
