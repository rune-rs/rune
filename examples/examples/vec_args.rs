use std::sync::Arc;

use rune::{Diagnostics, Options, Sources};
use runestick::{Context, Module, Source, Value, Vm, VmError};

fn main() -> runestick::Result<()> {
    let mut my_module = Module::with_item(&["mymodule"]);
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

    let mut diagnostics = Diagnostics::without_warnings();

    let unit = rune::load_sources(&context, &options, &mut sources, &mut diagnostics)?;

    let vm = Vm::new(Arc::new(context.runtime()), Arc::new(unit));
    let _ = vm.execute(&["main"], ())?.complete()?;

    Ok(())
}
