use rune::runtime::{VmError, VmErrorKind};
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Any, Context, ContextError, Diagnostics, Module, Source, Sources, Vm};
use std::sync::Arc;

#[derive(Any)]
struct External {
    #[rune(add_assign = "External::value_add_assign")]
    value: i64,
}

#[allow(clippy::unnecessary_lazy_evaluations)]
impl External {
    fn value_add_assign(&mut self, other: i64) -> Result<(), VmError> {
        self.value = self
            .value
            .checked_add(other)
            .ok_or_else(|| VmErrorKind::Overflow)?;

        Ok(())
    }
}

fn main() -> rune::Result<()> {
    let m = module()?;

    let mut context = Context::default();
    context.install(&m)?;
    let runtime = Arc::new(context.runtime());

    let mut sources = Sources::new();
    sources.insert(Source::new(
        "test",
        r#"pub fn main(external) { external.value += 1; }"#,
    ));

    let mut diagnostics = Diagnostics::new();

    let result = rune::prepare(&context, &mut sources)
        .with_diagnostics(&mut diagnostics)
        .build();

    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        diagnostics.emit(&mut writer, &sources)?;
    }

    let unit = result?;

    let mut vm = Vm::new(runtime, Arc::new(unit));

    let input = External {
        value: i64::max_value(),
    };
    let err = vm.call(&["main"], (input,)).unwrap_err();
    let (kind, _) = err.as_unwound();
    println!("{:?}", kind);
    Ok(())
}

fn module() -> Result<Module, ContextError> {
    let mut m = Module::new();
    m.ty::<External>()?;
    Ok(m)
}
