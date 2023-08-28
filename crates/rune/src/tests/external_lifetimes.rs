prelude!();

use std::sync::Arc;

use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use rune::{prepare, sources};
use rune::{AnyRef, Context, ContextError, Module, Vm};

#[test]
fn struct_with_lifetime() {
    #[derive(AnyRef)]
    pub struct Factory<'c> {
        #[rune(get)]
        pub widgets: &'c Total,
    }

    #[derive(Any)]
    pub struct Total(#[rune(get)] i64);

    fn make_module() -> Result<Module, ContextError> {
        let mut module = Module::new();
        module.ty::<Factory>()?;
        module.ty::<Total>()?;
        Ok(module)
    }

    let mut context = Context::with_default_modules().expect("Context should build");

    let m = make_module().expect("Module should be buildable");
    context.install(m).expect("Module should be installed");

    let runtime = Arc::new(context.runtime());

    let mut sources = sources! {
        entry => {
            pub fn main(factory) {
                factory.widgets.0
            }
        }
    };

    let mut diagnostics = Diagnostics::default();

    let unit = prepare(&mut sources)
        .with_diagnostics(&mut diagnostics)
        .with_context(&context)
        .build();

    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        diagnostics.emit(&mut writer, &sources).unwrap();
    }

    let unit = unit.expect("Unit should build");
    let mut vm = Vm::new(runtime, Arc::new(unit));

    let total = Total(25);
    let factory = Factory { widgets: &total };
    let value = vm.call(["main"], (&factory,)).unwrap();
    let output: i64 = rune::from_value(value).unwrap();
    assert_eq!(output, 25);
}

#[test]
fn tuple_with_lifetime() {
    #[derive(AnyRef)]
    pub struct Factory<'c>(#[rune(get)] &'c Total);

    #[derive(Any)]
    pub struct Total(#[rune(get)] i64);

    fn make_module() -> Result<Module, ContextError> {
        let mut module = Module::new();
        module.ty::<Factory>()?;
        module.ty::<Total>()?;
        Ok(module)
    }

    let mut context = Context::with_default_modules().expect("Context should build");

    let m = make_module().expect("Module should be buildable");
    context.install(m).expect("Module should be installed");

    let runtime = Arc::new(context.runtime());

    let mut sources = sources! {
        entry => {
            pub fn main(factory) {
                let total = factory.0;
                total.0
            }
        }
    };

    let mut diagnostics = Diagnostics::default();

    let unit = prepare(&mut sources)
        .with_diagnostics(&mut diagnostics)
        .with_context(&context)
        .build();

    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        diagnostics.emit(&mut writer, &sources).unwrap();
    }

    let unit = unit.expect("Unit should build");
    let mut vm = Vm::new(runtime, Arc::new(unit));

    let total = Total(25);
    let factory = Factory(&total);
    let value = vm.call(["main"], (&factory,)).unwrap();
    let output: i64 = rune::from_value(value).unwrap();
    assert_eq!(output, 25);
}

#[test]
fn struct_with_mut_lifetime() {
    #[derive(AnyRef)]
    pub struct Factory<'c> {
        #[rune(get)]
        pub widgets: &'c mut Total,
    }

    #[derive(Any)]
    pub struct Total(#[rune(get)] i64);

    impl Total {
        #[rune::function]
        fn add(&mut self, amount: i64) {
            self.0 += amount;
        }
    }

    fn make_module() -> Result<Module, ContextError> {
        let mut module = Module::new();
        module.ty::<Factory>()?;
        module.ty::<Total>()?;
        module.function_meta(Total::add)?;
        Ok(module)
    }

    let mut context = Context::with_default_modules().expect("Context should build");

    let m = make_module().expect("Module should be buildable");
    context.install(m).expect("Module should be installed");

    let runtime = Arc::new(context.runtime());

    let mut sources = sources! {
        entry => {
            pub fn main(factory) {
                factory.widgets.add(25);
                factory.widgets.0
            }
        }
    };

    let mut diagnostics = Diagnostics::default();

    let unit = prepare(&mut sources)
        .with_diagnostics(&mut diagnostics)
        .with_context(&context)
        .build();

    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        diagnostics.emit(&mut writer, &sources).unwrap();
    }

    let unit = unit.expect("Unit should build");
    let mut vm = Vm::new(runtime, Arc::new(unit));

    let mut total = Total(25);

    let mut factory = Factory {
        widgets: &mut total,
    };

    vm.call(["main"], (&mut factory,)).unwrap();
    assert_eq!(total.0, 50);
}

#[test]
fn nested_lifetimes() {
    #[derive(AnyRef)]
    pub struct Factory<'c> {
        #[rune(get_ref)]
        pub config: &'c Config<'c>,
    }

    #[derive(AnyRef)]
    pub struct Config<'c> {
        #[rune(get)]
        count: &'c Counter,
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
        module.ty::<Config>()?;

        Ok(module)
    }

    let mut context = Context::with_default_modules().expect("Context should build");

    let m = make_module().expect("Module should be buildable");
    context.install(m).expect("Module should be installed");

    let runtime = Arc::new(context.runtime());

    let mut sources = sources! {
        entry => {
            pub fn main(factory) {
                factory.config.count.count
            }
        }
    };

    let mut diagnostics = Diagnostics::default();

    let unit = prepare(&mut sources)
        .with_diagnostics(&mut diagnostics)
        .with_context(&context)
        .build();

    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        diagnostics.emit(&mut writer, &sources).unwrap();
    }

    let unit = unit.expect("Unit should build");

    let mut vm = Vm::new(runtime, Arc::new(unit));

    let count = Counter { count: 100 };
    let config = Config { count: &count };
    let factory = Factory { config: &config };

    let value = vm.call(["main"], (&factory,)).unwrap();
    let output: i64 = rune::from_value(value).unwrap();
    assert_eq!(output, 100);
}
