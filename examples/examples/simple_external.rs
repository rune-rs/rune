use rune::runtime::Vm;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Any, ContextError, Diagnostics, Module};

use std::sync::Arc;

#[derive(Debug, Any)]
struct External {
    value: u32,
}

pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new();
    module.ty::<External>()?;
    Ok(module)
}

fn main() -> rune::support::Result<()> {
    let m = module()?;

    let mut context = rune_modules::default_context()?;
    context.install(m)?;
    let runtime = Arc::new(context.runtime()?);

    let mut sources = rune::sources! {
        entry => {
            pub fn main(external) {
                external
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

    let output = vm.call(["main"], (External { value: 42 },))?;
    let output: External = rune::from_value(output)?;
    println!("{:?}", output);
    assert_eq!(output.value, 42);
    Ok(())
}
