//! This example showcases a more complicated enum with multiple variants.
//!
//! It originates from https://github.com/rune-rs/rune/issues/905

prelude!();

use crate::alloc::String;

#[derive(Any, TryClone, Debug, PartialEq, Eq)]
#[rune(item = ::module)]
pub enum Enum {
    #[rune(constructor)]
    Named {
        #[rune(get, set)]
        field: String,
    },
    #[rune(constructor)]
    Unnamed(#[rune(get, set)] String),
    #[rune(constructor)]
    UnnamedEmpty(),
    #[rune(constructor)]
    Empty,
}

#[test]
fn enum_variants() -> Result<()> {
    let m = module()?;

    let mut context = Context::with_default_modules()?;
    context.install(m)?;

    let runtime = Arc::new(context.runtime()?);

    let mut sources = sources! {
        entry => {
            use module::Enum;

            pub fn named() {
                Enum::Named {
                    field: "Hello World"
                }
            }

            pub fn unnamed() {
                Enum::Unnamed("Hello World")
            }

            pub fn unnamed_empty() {
                Enum::UnnamedEmpty()
            }

            pub fn empty() {
                Enum::Empty
            }
        }
    };

    let mut diagnostics = Diagnostics::new();

    let unit = prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build()?;

    let mut vm = Vm::new(runtime, Arc::new(unit));

    let output = vm.call(["named"], ())?;
    let output: Enum = from_value(output)?;
    assert_eq!(
        output,
        Enum::Named {
            field: String::try_from("Hello World")?
        }
    );

    let output = vm.call(["unnamed"], ())?;
    let output: Enum = from_value(output)?;
    assert_eq!(output, Enum::Unnamed(String::try_from("Hello World")?));

    let output = vm.call(["unnamed_empty"], ())?;
    let output: Enum = from_value(output)?;
    assert_eq!(output, Enum::UnnamedEmpty());

    let output = vm.call(["empty"], ())?;
    let output: Enum = from_value(output)?;
    assert_eq!(output, Enum::Empty);
    Ok(())
}

fn module() -> Result<Module, ContextError> {
    let mut m = Module::with_crate("module")?;
    m.ty::<Enum>()?;
    Ok(m)
}
