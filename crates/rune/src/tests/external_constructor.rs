prelude!();

use std::sync::Arc;

use rune::{from_value, prepare, sources};
use rune::{Any, Context, ContextError, Module, Vm};

/// Tests pattern matching and constructing over an external variant from within
/// Rune.
#[test]
fn construct_enum() {
    #[derive(Debug, Any, PartialEq, Eq)]
    enum Enum {
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

    fn make_module() -> Result<Module, ContextError> {
        let mut module = Module::new();
        module.ty::<Enum>()?;
        Ok(module)
    }

    let m = make_module().expect("Module should be buildable");

    let mut context = Context::new();
    context.install(m).expect("Context should build");
    let runtime = Arc::new(context.runtime());

    let mut sources = sources! {
        entry => {
            pub fn main(external) {
                match external {
                    Enum::First(value) => Enum::Output(value * 1),
                    Enum::Second(value) => Enum::Output(value * 2),
                    Enum::Third => Enum::Output(3),
                    Enum::Fourth { a, b } => Enum::Output((a * b) * 4),
                    _ => 0,
                }
            }
        }
    };

    let unit = prepare(&mut sources)
        .with_context(&context)
        .build()
        .expect("Unit should build");

    let mut vm = Vm::new(runtime, Arc::new(unit));

    let output = vm.call(["main"], (Enum::First(42),)).unwrap();
    let output: Enum = from_value(output).unwrap();
    assert_eq!(output, Enum::Output(42));

    let output = vm.call(["main"], (Enum::Second(43),)).unwrap();
    let output: Enum = from_value(output).unwrap();
    assert_eq!(output, Enum::Output(43 * 2));

    let output = vm.call(["main"], (Enum::Third,)).unwrap();
    let output: Enum = from_value(output).unwrap();
    assert_eq!(output, Enum::Output(3));

    let output = vm.call(["main"], (Enum::Fourth { a: 2, b: 3 },)).unwrap();
    let output: Enum = from_value(output).unwrap();
    assert_eq!(output, Enum::Output(2 * 3 * 4));
}
