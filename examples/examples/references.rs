use rune::runtime::Protocol;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Any, Diagnostics, Module, Vm};

use std::sync::Arc;

#[derive(Debug, Default, Any)]
struct Foo {
    value: i64,
}

impl Foo {
    fn add_assign(&mut self, value: i64) {
        self.value += value;
    }
}

#[allow(clippy::disallowed_names)]
fn main() -> rune::support::Result<()> {
    let mut module = Module::new();
    module.ty::<Foo>()?;
    module.associated_function(Protocol::ADD_ASSIGN, Foo::add_assign)?;

    let mut context = rune_modules::default_context()?;
    context.install(module)?;

    let runtime = Arc::new(context.runtime()?);

    let mut sources = rune::sources! {
        entry => {
            pub fn main(number) {
                number += 1;
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

    let mut vm = Vm::new(runtime, Arc::new(unit));

    let mut foo = Foo::default();
    let _ = vm.call(["main"], (&mut foo,))?;
    println!("{:?}", foo);
    Ok(())
}
