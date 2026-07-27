//! Demonstrates statics which are declared with the build rather than by the
//! script.
//!
//! A host which owns a piece of state usually doesn't want the script to be
//! responsible for declaring it. Declaring it with the build makes it available
//! to every source in the unit without them saying anything about it, in the
//! same way that a native module makes a function available.

use rune::runtime::Globals;
use rune::sync::Arc;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Diagnostics, Statics, Vm};

fn main() -> rune::support::Result<()> {
    let context = rune_modules::default_context()?;
    let runtime = Arc::try_new(context.runtime()?)?;

    // Note that the script uses `GREETING` and `CALLS` without declaring them.
    let mut sources = rune::sources!(
        entry => {
            pub fn main(name) {
                CALLS += 1;
                format!("{GREETING}, {name}! (call {CALLS})")
            }
        }
    );

    let mut statics = Statics::new();
    statics.insert(["GREETING"])?;
    statics.insert(["CALLS"])?;

    let mut diagnostics = Diagnostics::new();

    let result = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .with_statics(&statics)
        .build();

    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        diagnostics.emit(&mut writer, &sources)?;
    }

    let unit = Arc::try_new(result?)?;

    // A declared static has no initializer, so both of these have to be
    // assigned before the script reads them.
    let globals = Globals::new(unit.clone())?;
    globals.set(["GREETING"], rune::to_value("Hello")?)?;
    globals.set(["CALLS"], rune::to_value(0i64)?)?;

    let mut vm = Vm::new(runtime, unit).with_globals(globals.clone());

    for name in ["Jane", "John"] {
        let output = vm.call(["main"], (name,))?;
        let output: String = rune::from_value(output)?;
        println!("{output}");
    }

    let calls = globals.get(["CALLS"])?.expect("CALLS to be initialized");
    println!("calls: {}", rune::from_value::<i64>(calls)?);
    Ok(())
}
