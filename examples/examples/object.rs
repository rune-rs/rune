use rune::alloc;
use rune::runtime::Object;
use rune::sync::Arc;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Diagnostics, Vm};

fn main() -> rune::support::Result<()> {
    let context = rune_modules::default_context()?;
    let runtime = Arc::try_new(context.runtime()?)?;

    let mut sources = rune::sources! {
        entry => {
            pub fn calc(input) {
                dbg(input["key"]);
                input["key"] = "World";
                input
            }
        }
    };

    let mut diagnostics = Diagnostics::new();

    let result = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build();

    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        diagnostics.emit(&mut writer, &sources)?;
    }

    let unit = result?;
    let unit = Arc::try_new(unit)?;
    let mut vm = Vm::new(runtime, unit);

    let mut object = Object::new();
    object.insert(alloc::String::try_from("key")?, rune::to_value(42i64)?)?;

    let output = vm.call(["calc"], (object,))?;
    let output: Object = rune::from_value(output)?;

    println!("{:?}", output.get("key"));
    Ok(())
}
