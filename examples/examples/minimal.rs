use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Diagnostics, Vm};

use std::sync::Arc;

fn main() -> rune::support::Result<()> {
    let context = rune_modules::default_context()?;

    let mut sources = rune::sources!(
        entry => {
            pub fn main(number) {
                number + 10
            }
        }
    );

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

    let mut vm = Vm::new(Arc::new(context.runtime()?), Arc::new(unit));
    let output = vm.execute(["main"], (33i64,))?.complete().into_result()?;
    let output: i64 = rune::from_value(output)?;

    println!("output: {}", output);
    Ok(())
}
