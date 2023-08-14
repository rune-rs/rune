prelude!();

use std::sync::Arc;

use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use rune::{prepare, sources};
use rune::{Any, Context, ContextError, Module, Vm};

#[test]
fn test_external_references_struct() {
    #[derive(Any)]
    pub struct Factory<'c> {
        #[rune(get)]
        pub widgets: &'c mut Counter,
    }

    impl<'c> Factory<'c> {
        #[rune::function]
        fn combine(&mut self, other: &Factory) {
            self.widgets.count += other.widgets.count;
        }
    }

    #[derive(Any)]
    pub struct Counter {
        #[rune(get, set)]
        count: i64,
    }

    fn make_module() -> Result<Module, ContextError> {
        let mut module = Module::new();
        module.ty::<Factory>()?;
        module.ty::<Counter>()?;
        module.function_meta(Factory::combine)?;

        Ok(module)
    }

    let mut context = Context::with_default_modules().expect("Context should build");

    let m = make_module().expect("Module should be buildable");
    context.install(m).expect("Module should be installed");

    let runtime = Arc::new(context.runtime());

    let mut sources = sources! {
        entry => {
            pub fn main(first, second) {
                for _ in 0..10 {
                    first.widgets.count = first.widgets.count + 1;
                }

                for _ in 0..10 {
                    second.widgets.count = second.widgets.count + 1;
                }

                first.combine(second);
            }
        }
    };

    let mut diagnostics = Diagnostics::default();
    let unit = prepare(&mut sources)
        .with_diagnostics(&mut diagnostics)
        .with_context(&context)
        .build();

    let unit = match unit {
        Ok(unit) => unit,
        Err(err) => {
            if !diagnostics.is_empty() {
                let mut writer = StandardStream::stderr(ColorChoice::Always);
                diagnostics.emit(&mut writer, &sources).unwrap();
            }

            panic!("build failed: {:?}", err);
        }
    };

    let mut vm = Vm::new(runtime, Arc::new(unit));

    let mut first_count = Counter { count: 0 };

    let mut first = Factory {
        widgets: &mut first_count,
    };

    let mut second_count = Counter { count: 0 };

    let mut second = Factory {
        widgets: &mut second_count,
    };

    vm.call(["main"], (&mut first, &mut second)).unwrap();
    assert_eq!(first_count.count, 20);
}
