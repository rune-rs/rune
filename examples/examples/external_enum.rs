use rune::runtime::Vm;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Any, ContextError, Diagnostics, Module};

use std::sync::Arc;

#[derive(Debug, Any, PartialEq, Eq)]
enum External {
    #[rune(constructor)]
    First(#[rune(get)] u32),
    Second(#[rune(get)] u32, u32),
    #[rune(constructor)]
    Third,
    Fourth {
        #[rune(get)]
        a: u32,
        #[rune(get)]
        b: u32,
    },
    #[rune(constructor)]
    Output(#[rune(get)] u32),
}

fn main() -> rune::support::Result<()> {
    let m = module()?;

    let mut context = rune_modules::default_context()?;
    context.install(m)?;
    let runtime = Arc::new(context.runtime()?);

    let mut sources = rune::sources! {
        entry => {
            pub fn main(external) {
                match external {
                    External::First(value) => External::Output(value),
                    External::Second(a) => External::Output(2),
                    External::Third => External::Output(3),
                    External::Fourth { a, b } => External::Output((a * b) * 4),
                    _ => 0,
                }
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

    let output = vm.call(["main"], (External::First(42),))?;
    let output: External = rune::from_value(output)?;
    println!("{:?}", output);

    let output = vm.call(["main"], (External::Second(42, 12345),))?;
    let output: External = rune::from_value(output)?;
    println!("{:?}", output);

    let output = vm.call(["main"], (External::Third,))?;
    let output: External = rune::from_value(output)?;
    println!("{:?}", output);

    let output = vm.call(["main"], (External::Fourth { a: 42, b: 2 },))?;
    let output: External = rune::from_value(output)?;
    println!("{:?}", output);
    Ok(())
}

pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new();
    module.ty::<External>()?;
    Ok(module)
}
