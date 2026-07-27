//! Injecting a host-owned registry into scripts through a bare `static`.
//!
//! The registry is never passed as an argument. The script declares
//! `static REGISTRY;` with no initializer, the host writes it into the
//! [`Globals`] storage once, and every function in the script can reach it.

use std::collections::HashMap;

use rune::runtime::Globals;
use rune::sync::Arc;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Any, ContextError, Diagnostics, Module, Vm};

/// A registry owned by the host and shared with every script that runs.
#[derive(Debug, Any)]
struct Registry {
    settings: HashMap<String, i64>,
}

impl Registry {
    /// Look up a setting, returning `None` if it isn't present.
    #[rune::function]
    fn get(&self, key: &str) -> Option<i64> {
        self.settings.get(key).copied()
    }
}

fn module() -> Result<Module, ContextError> {
    let mut m = Module::new();
    m.ty::<Registry>()?;
    m.function_meta(Registry::get)?;
    Ok(m)
}

fn main() -> rune::support::Result<()> {
    let mut context = rune_modules::default_context()?;
    context.install(module()?)?;
    let runtime = Arc::try_new(context.runtime()?)?;

    // Note that `REGISTRY` is not a parameter of either function.
    let mut sources = rune::sources!(
        entry => {
            static REGISTRY;

            fn describe(key) {
                match REGISTRY.get(key) {
                    Some(value) => format!("{key} is {value}"),
                    None => format!("{key} is unset"),
                }
            }

            pub fn main() {
                [describe("width"), describe("height"), describe("depth")]
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

    let registry = Registry {
        settings: HashMap::from([("width".to_owned(), 800), ("height".to_owned(), 600)]),
    };

    // Wire the registry into the slot the script declared, before it runs.
    let globals = Globals::new(unit.clone())?;
    globals.set(["REGISTRY"], rune::to_value(registry)?)?;

    let mut vm = Vm::new(runtime, unit).with_globals(globals);

    let output: Vec<String> = rune::from_value(vm.call(["main"], ())?)?;

    for line in output {
        println!("{line}");
    }

    Ok(())
}
