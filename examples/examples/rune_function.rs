use rune::runtime::Function;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Diagnostics, EmitDiagnostics, FromValue, Source, Sources, Vm};
use std::sync::Arc;

fn main() -> rune::Result<()> {
    let context = rune_modules::default_context()?;
    let runtime = Arc::new(context.runtime());

    let mut sources = Sources::new();
    sources.insert(Source::new(
        "test",
        r#"
        fn foo(a, b) {
            a + b
        }

        pub fn main() {
            foo
        }
        "#,
    ));

    let mut diagnostics = Diagnostics::new();

    let result = rune::prepare(&context, &mut sources)
        .with_diagnostics(&mut diagnostics)
        .build();

    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        diagnostics.emit_diagnostics(&mut writer, &sources)?;
    }

    let unit = result?;

    let mut vm = Vm::new(runtime, Arc::new(unit));
    let output = vm.call(&["main"], ())?;
    let output = Function::from_value(output)?;

    println!("{}", output.call::<(i64, i64), i64>((1, 3))?);
    println!("{}", output.call::<(i64, i64), i64>((2, 6))?);
    Ok(())
}
