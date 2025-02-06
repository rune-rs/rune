//! This example showcases a more complicated enum with multiple variants.
//!
//! It originates from https://github.com/rune-rs/rune/issues/905

use std::sync::Arc;

use rune::alloc::prelude::*;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Any, Context, ContextError, Diagnostics, Module, Vm};

#[derive(Any, TryClone, Debug, PartialEq, Eq)]
#[rune(item = ::ddl)]
pub enum Enum {
    #[rune(constructor)]
    Struct {
        #[rune(get, set)]
        field: String,
    },
}

fn main() -> rune::support::Result<()> {
    let m = module()?;

    let mut context = Context::with_default_modules()?;
    context.install(m)?;

    let runtime = Arc::new(context.runtime()?);

    let mut sources = rune::sources! {
        entry => {
            use ddl::Enum;

            pub fn main() {
                Enum::Struct {
                    field: "Hello World"
                }
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

    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        diagnostics.emit_detailed(&mut writer, &sources, &unit, &context)?;
    }

    let mut vm = Vm::new(runtime, Arc::new(unit));
    let output = vm.call(["main"], ())?;
    let output: Enum = rune::from_value(output)?;

    vm.with(|| println!("{output:?}"));
    Ok(())
}

fn module() -> Result<Module, ContextError> {
    let mut m = Module::with_crate("ddl")?;
    m.ty::<Enum>()?;
    Ok(m)
}
