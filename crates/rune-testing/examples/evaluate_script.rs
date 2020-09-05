use rune::termcolor::{ColorChoice, StandardStream};
use rune::EmitDiagnostics as _;
use runestick::{FromValue as _, Item, Source, Vm};

use std::error::Error;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let source = Source::new(
        "script",
        r#"
        fn calculate(a, b) {
            println("Hello World");
            a + b
        }
        "#,
    );

    let context = Arc::new(rune::default_context()?);
    let options = rune::Options::default();
    let mut warnings = rune::Warnings::new();

    let unit = match rune::load_source(&*context, &options, source, &mut warnings) {
        Ok(unit) => unit,
        Err(error) => {
            let mut writer = StandardStream::stderr(ColorChoice::Always);
            error.emit_diagnostics(&mut writer)?;
            return Ok(());
        }
    };

    if !warnings.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        rune::emit_warning_diagnostics(&mut writer, &warnings, &unit)?;
    }

    let vm = Vm::new(context.clone(), Arc::new(unit));

    let mut execution = vm.call(Item::of(&["calculate"]), (10i64, 20i64))?;
    let value = execution.async_complete().await?;

    let value = i64::from_value(value)?;

    println!("{}", value);
    Ok(())
}
