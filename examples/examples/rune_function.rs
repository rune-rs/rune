use rune::runtime::Function;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Diagnostics, Vm};

use std::sync::Arc;

fn main() -> rune::support::Result<()> {
    let context = rune_modules::default_context()?;
    let runtime = Arc::new(context.runtime()?);

    let mut sources = rune::sources! {
        entry => {
            fn foo(a, b) {
                a + b
            }

            pub fn main() {
                foo
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
    let output: Function = rune::from_value(output)?;

    println!("{}", output.call::<(i64, i64), i64>((1, 3)).into_result()?);
    println!("{}", output.call::<(i64, i64), i64>((2, 6)).into_result()?);
    Ok(())
}
