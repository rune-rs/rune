use rune::termcolor::{ColorChoice, StandardStream};
use rune::{EmitDiagnostics as _, Options, Sources, Warnings};
use runestick::{FromValue as _, Source, Vm};

use std::error::Error;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let context = Arc::new(rune::default_context()?);
    let options = Options::default();
    let mut warnings = Warnings::new();
    let mut sources = Sources::new();

    sources.insert(Source::new(
        "script",
        r#"
        fn calculate(a, b) {
            println("Hello World");
            a + b
        }
        "#,
    ));

    let unit = match rune::load_sources(&*context, &options, &mut sources, &mut warnings) {
        Ok(unit) => unit,
        Err(error) => {
            let mut writer = StandardStream::stderr(ColorChoice::Always);
            error.emit_diagnostics(&mut writer, &sources)?;
            return Ok(());
        }
    };

    if !warnings.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        warnings.emit_diagnostics(&mut writer, &sources)?;
    }

    let vm = Vm::new(context.clone(), Arc::new(unit));

    let mut execution = vm.execute(&["calculate"], (10i64, 20i64))?;
    let value = execution.async_complete().await?;

    let value = i64::from_value(value)?;

    println!("{}", value);
    Ok(())
}
