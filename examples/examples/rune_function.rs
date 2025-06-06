use rune::runtime::Function;
use rune::sync::Arc;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Diagnostics, Vm};

fn main() -> rune::support::Result<()> {
    let context = rune_modules::default_context()?;
    let runtime = Arc::try_new(context.runtime()?)?;

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
    let unit = Arc::try_new(unit)?;
    let mut vm = Vm::new(runtime, unit);
    let output = vm.call(["main"], ())?;
    let output: Function = rune::from_value(output)?;

    println!("{}", output.call::<i64>((1, 3))?);
    println!("{}", output.call::<i64>((2, 6))?);
    Ok(())
}
