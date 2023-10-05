use rune::alloc::Vec;
use rune::runtime::{Function, VmResult};
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{ContextError, Diagnostics, Module, Value, Vm};

use std::sync::Arc;

fn main() -> rune::support::Result<()> {
    let m = module()?;

    let mut context = rune_modules::default_context()?;
    context.install(m)?;
    let runtime = Arc::new(context.runtime()?);

    let mut sources = rune::sources! {
        entry => {
            pub fn main() {
                mymodule::pass_along(add, [5, 9])
            }

            fn add(a, b) {
                a + b
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

    let mut vm = Vm::new(runtime, Arc::new(unit));
    let output = vm.call(["main"], ())?;
    let output: u32 = rune::from_value(output)?;

    println!("{}", output);
    Ok(())
}

fn module() -> Result<Module, ContextError> {
    let mut m = Module::with_item(["mymodule"])?;

    m.function(
        "pass_along",
        |func: Function, args: Vec<Value>| -> VmResult<Value> { func.call(args) },
    )
    .build()?;

    Ok(m)
}
