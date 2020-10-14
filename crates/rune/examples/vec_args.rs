use std::sync::Arc;

use rune::{Errors, Options, Sources, Warnings};
use runestick::{Context, Module, Source, Value, Vm, VmError};

fn main() -> runestick::Result<()> {
    let mut my_module = Module::new(&["mymodule"]);
    my_module.function(
        &["pass_along"],
        |func: runestick::Function, args: Vec<Value>| -> Result<Value, VmError> { func.call(args) },
    )?;

    let mut context = Context::with_default_modules()?;
    context.install(&my_module)?;

    let options = Options::default();

    let mut sources = Sources::new();
    sources.insert(Source::new(
        "test",
        r#"
        pub fn main() {
            let value = mymodule::pass_along(add, [5, 9]);
            println(`${value}`);
        }

        fn add(a, b) {
            a + b
        }
        "#,
    ));

    let mut errors = Errors::new();
    let mut warnings = Warnings::disabled();

    let unit = rune::load_sources(&context, &options, &mut sources, &mut errors, &mut warnings)?;

    let vm = Vm::new(Arc::new(context), Arc::new(unit));
    let _ = vm.execute(&["main"], ())?.complete()?;

    Ok(())
}
