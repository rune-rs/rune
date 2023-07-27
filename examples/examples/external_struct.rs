use std::sync::Arc;

use rune::runtime::Vm;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Any, ContextError, Diagnostics, Module};

#[derive(Default, Debug, Any, PartialEq, Eq)]
#[rune(constructor)]
struct External {
    #[rune(get, set)]
    suite_name: String,
    #[rune(get, set)]
    room_number: usize,
}

fn main() -> rune::Result<()> {
    let m = module()?;

    let mut context = rune_modules::default_context()?;
    context.install(m)?;
    let runtime = Arc::new(context.runtime());

    let mut sources = rune::sources! {
        entry => {
            pub fn main() {
                let external = External {
                    suite_name: "Fowler",
                    room_number: 1300,
                };

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

    let output = vm.call(["main"], ())?;
    let output: External = rune::from_value(output)?;
    println!("{:?}", output);

    Ok(())
}

pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new();
    module.ty::<External>()?;
    Ok(module)
}
