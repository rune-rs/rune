use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Diagnostics, EmitDiagnostics as _, Options, Sources};
use runestick::{FromValue as _, Source, Vm};

use std::error::Error;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let context = rune_modules::default_context()?;
    let options = Options::default();
    let mut sources = Sources::new();

    sources.insert(Source::new(
        "script",
        r#"
        pub fn calculate(a, b) {
            println("Hello World");
            a + b
        }
        "#,
    ));

    let mut diagnostics = Diagnostics::new();

    let result = rune::load_sources(&context, &options, &mut sources, &mut diagnostics);

    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        diagnostics.emit_diagnostics(&mut writer, &sources)?;
    }

    let unit = result?;
    let mut vm = Vm::new(Arc::new(context.runtime()), Arc::new(unit));

    let mut execution = vm.execute(&["calculate"], (10i64, 20i64))?;
    let value = execution.async_complete().await?;

    let value = i64::from_value(value)?;

    println!("{}", value);
    Ok(())
}
