use std::sync::Arc;

use rune::termcolor::{ColorChoice, StandardStream};
use rune::Any;
use rune::{ContextError, Diagnostics, FromValue, Module, Vm};

fn main() -> rune::Result<()> {
    let m = module()?;

    let mut context = rune_modules::default_context()?;
    context.install(m)?;

    let runtime = Arc::new(context.runtime());

    let mut sources = rune::sources! {
        entry => {
            pub fn main(a) {
                add(a)
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
    let output = vm.call(["main"], (1u32,))?;
    let output = i64::from_value(output)?;

    println!("{}", output);
    Ok(())
}

/// Add `1` to the current argument.
#[rune::function]
fn add(value: i64) -> i64 {
    value + 1
}

/// Add `1` asynchronously to the current argument.
#[rune::function]
async fn add_async(value: i64) -> i64 {
    value + 1
}

#[derive(Any)]
struct Test {
    field: i64,
}

impl Test {
    #[rune::function]
    fn add(&self, value: i64) -> i64 {
        self.field + value
    }

    #[rune::function]
    async fn add_async(&self, value: i64) -> i64 {
        self.field + value
    }
}

fn module() -> Result<Module, ContextError> {
    let mut m = Module::new();
    m.function2(add)?;
    m.function2(add_async)?;
    m.ty::<Test>()?;
    m.function2(Test::add)?;
    m.function2(Test::add_async)?;
    Ok(m)
}
