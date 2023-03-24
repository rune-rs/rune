use std::sync::Arc;

use rune::runtime::Vm;
use rune::{Any, Context, ContextError, FromValue, Module};

/// Tests pattern matching and constructing over an external variant from within
/// Rune.
#[test]
fn test_external_enum() -> Result<(), Box<dyn std::error::Error>> {
    #[derive(Debug, Any, PartialEq, Eq)]
    enum External {
        #[rune(constructor)]
        First(#[rune(get)] u32),
        #[rune(constructor)]
        Second(#[rune(get)] u32),
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

    pub fn module() -> Result<Module, ContextError> {
        let mut module = Module::new();
        module.ty::<External>()?;
        Ok(module)
    }

    let m = module()?;

    let mut context = Context::new();
    context.install(m)?;
    let runtime = Arc::new(context.runtime());

    let mut sources = rune::sources! {
        entry => {
            pub fn main(external) {
                match external {
                    External::First(value) => External::Output(value * 1),
                    External::Second(value) => External::Output(value * 2),
                    External::Third => External::Output(3),
                    External::Fourth { a, b } => External::Output((a * b) * 4),
                    _ => 0,
                }
            }
        }
    };

    let unit = rune::prepare(&mut sources).with_context(&context).build()?;

    let mut vm = Vm::new(runtime, Arc::new(unit));

    let output = vm.call(["main"], (External::First(42),))?;
    let output = External::from_value(output)?;
    assert_eq!(output, External::Output(42));

    let output = vm.call(["main"], (External::Second(43),))?;
    let output = External::from_value(output)?;
    assert_eq!(output, External::Output(43 * 2));

    let output = vm.call(["main"], (External::Third,))?;
    let output = External::from_value(output)?;
    assert_eq!(output, External::Output(3));

    let output = vm.call(["main"], (External::Fourth { a: 2, b: 3 },))?;
    let output = External::from_value(output)?;
    assert_eq!(output, External::Output(2 * 3 * 4));
    Ok(())
}
