use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Any, Context, Diagnostics, EmitDiagnostics, Module, Protocol, Source, Sources, Vm};
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

#[allow(clippy::blacklisted_name)]
fn main() -> rune::Result<()> {
    let mut module = Module::new();
    module.ty::<Foo>()?;
    module.inst_fn(Protocol::ADD_ASSIGN, Foo::add_assign)?;

    let mut context = Context::with_default_modules()?;
    context.install(&module)?;

    let runtime = Arc::new(context.runtime());

    let mut sources = Sources::new();
    sources.insert(Source::new(
        "test",
        r#"
        pub fn main(number) {
            number += 1;
        }
        "#,
    ));

    let mut diagnostics = Diagnostics::new();

    let result = rune::prepare(&context, &mut sources)
        .with_diagnostics(&mut diagnostics)
        .build();

    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        diagnostics.emit_diagnostics(&mut writer, &sources)?;
    }

    let unit = result?;

    let mut vm = Vm::new(runtime, Arc::new(unit));

    let mut foo = Foo::default();
    let _ = vm.call(&["main"], (&mut foo,))?;
    println!("{:?}", foo);
    Ok(())
}
