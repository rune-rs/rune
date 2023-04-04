use rune::termcolor::{ColorChoice, StandardStream};
use rune::{ContextError, Diagnostics, Module, Vm};
use std::sync::Arc;

fn divide_by_three(value: i64) -> i64 {
    value / 3
}

#[tokio::main]
async fn main() -> rune::Result<()> {
    let m = module()?;

    let mut context = rune_modules::default_context()?;
    context.install(m)?;
    let runtime = Arc::new(context.runtime());

    let mut sources = rune::sources!(entry => {
        pub fn main(number) {
            number.divide_by_three()
        }
    });

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

    let mut vm = Vm::new(runtime, Arc::new(unit));
    let output = vm.execute(["main"], (33i64,))?.complete().into_result()?;
    let output: i64 = rune::from_value(output)?;

    println!("output: {}", output);
    Ok(())
}

fn module() -> Result<Module, ContextError> {
    let mut m = Module::with_item(["mymodule"]);
    m.inst_fn("divide_by_three", divide_by_three)?;
    Ok(m)
}
