use rune::runtime::Object;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Diagnostics, FromValue, Source, Sources, Value, Vm};
use std::sync::Arc;

fn main() -> rune::Result<()> {
    let context = rune_modules::default_context()?;
    let runtime = Arc::new(context.runtime());

    let mut sources = Sources::new();
    sources.insert(Source::new(
        "entry",
        r#"
        pub fn calc(input) {
            dbg(input["key"]);
            input["key"] = "World";
            input
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

    let mut object = Object::new();
    object.insert(String::from("key"), Value::from(42i64));

    let output = vm.call(&["calc"], (object,))?;
    let output = Object::from_value(output)?;

    println!("{:?}", output.get("key"));
    Ok(())
}
