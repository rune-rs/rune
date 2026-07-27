//! Demonstrates `static` items and the storage which backs them.
//!
//! The storage is configured on the virtual machine separately from the unit,
//! so the caller can seed a static before any script runs and read back what
//! the script left behind afterwards.

use rune::runtime::Globals;
use rune::sync::Arc;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Diagnostics, Vm};

fn main() -> rune::support::Result<()> {
    let context = rune_modules::default_context()?;
    let runtime = Arc::try_new(context.runtime()?)?;

    // `GREETING` has no initializer, so the caller has to supply it before the
    // script reads it. `CALLS` has one, which is lazily evaluated on the first
    // read.
    let mut sources = rune::sources!(
        entry => {
            static GREETING;
            static CALLS = 0;

            pub fn main(name) {
                CALLS += 1;
                format!("{GREETING}, {name}! (call {CALLS})")
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

    let unit = Arc::try_new(result?)?;

    let globals = Globals::new(unit.clone())?;
    globals.set(["GREETING"], rune::to_value("Hello")?)?;

    // The storage is a handle, so this clone shares the slots with the vm.
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
