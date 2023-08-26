use std::sync::Arc;

use rune::sources;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Any, Context, ContextError, Diagnostics, Module, Vm};

#[derive(Any)]
pub struct Factory<'c> {
    pub widgets: &'c u32,
}

impl<'c> Factory<'c> {
    #[rune::function(instance, path = Self::escape)]
    fn escape(this: &Factory<'static>) {
        let widgets = this.widgets;

        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(1));
            dbg!(widgets);
        });
    }
}

fn make_module() -> Result<Module, ContextError> {
    let mut module = Module::new();
    module.ty::<Factory>()?;
    module.function_meta(Factory::escape)?;
    Ok(module)
}

fn main() {
    let mut context = Context::with_default_modules().expect("Context should build");

    let m = make_module().expect("Module should be buildable");
    context.install(m).expect("Module should be installed");

    let runtime = Arc::new(context.runtime());

    let mut sources = sources! {
        entry => {
            pub fn main(first) {
                first.escape();
            }
        }
    };

    let mut diagnostics = Diagnostics::default();

    let unit = rune::prepare(&mut sources)
        .with_diagnostics(&mut diagnostics)
        .with_context(&context)
        .build();

    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        diagnostics.emit(&mut writer, &sources).unwrap();
    }

    let unit = unit.expect("build failed");

    let mut vm = Vm::new(runtime, Arc::new(unit));

    {
        let widgets = 42;
        let first = Factory { widgets: &widgets };
        vm.call(["main"], (&first,)).unwrap();
    }

    std::thread::sleep(std::time::Duration::from_secs(2));
}
