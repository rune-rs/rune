use rune::runtime::VmError;
use rune::sync::Arc;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Any, ContextError, Diagnostics, Module, Vm};

#[derive(Any)]
struct External {
    #[rune(add_assign = External::value_add_assign)]
    value: i64,
}

#[allow(clippy::unnecessary_lazy_evaluations)]
impl External {
    fn value_add_assign(&mut self, other: i64) -> Result<(), VmError> {
        self.value = self
            .value
            .checked_add(other)
            .ok_or_else(VmError::overflow)?;
        Ok(())
    }
}

fn main() -> rune::support::Result<()> {
    let m = module()?;

    let mut context = rune_modules::default_context()?;
    context.install(m)?;

    let runtime = Arc::try_new(context.runtime()?)?;

    let mut sources = rune::sources! {
        entry => {
            pub fn main(e) {
                e.value += 1;
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
    let unit = Arc::try_new(unit)?;

    let mut vm = Vm::new(runtime, unit);

    let input = External { value: i64::MAX };
    let err = vm.call(["main"], (input,)).unwrap_err();
    println!("{err:?}");
    Ok(())
}

fn module() -> Result<Module, ContextError> {
    let mut m = Module::new();
    m.ty::<External>()?;
    Ok(m)
}
