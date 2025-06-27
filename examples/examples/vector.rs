use rune::sync::Arc;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Diagnostics, Vm};

fn main() -> rune::support::Result<()> {
    let context = rune_modules::default_context()?;
    let runtime = Arc::try_new(context.runtime()?)?;

    let mut sources = rune::sources! {
        entry => {
            pub fn calc(input) {
                let output = 0;

                for value in input {
                    output += value;
                }

                [output]
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
    let unit = Arc::try_new(unit)?;
    let mut vm = Vm::new(runtime, unit);

    let input = vec![1, 2, 3, 4];
    let output = vm.call(["calc"], (input,))?;
    let output: Vec<i64> = rune::from_value(output)?;

    println!("{output:?}");
    Ok(())
}
