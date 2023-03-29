use std::sync::Arc;

use rune::runtime::Vm;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Any, ContextError, Diagnostics, FromValue, Module};

#[derive(Debug)]
struct Inner {
    foo: String,
    bar: String,
}

#[derive(Debug, Any)]
struct External {
    map: Inner,
}

impl External {
    async fn foo(&mut self) -> &str {
        self.map.foo.as_ref()
    }

    async fn bar(&mut self) -> &str {
        self.map.bar.as_ref()
    }
}

pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new();
    module.ty::<External>()?;
    module.async_inst_fn("foo", External::foo)?;
    module.async_inst_fn("bar", External::bar)?;
    Ok(module)
}

#[tokio::main]
async fn main() -> rune::Result<()> {
    let m = module()?;

    let mut context = rune_modules::default_context()?;
    context.install(m)?;
    let runtime = Arc::new(context.runtime());

    let mut sources = rune::sources! {
        entry => {
            use std::future;

            pub async fn main(external) {
                let [foo, bar] = future::join([
                    external.foo(),
                    external.bar(),
                ]).await;

                foo + ", " + bar
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

    let map = Inner {
        foo: "foo".to_string(),
        bar: "bar".to_string(),
    };
    let external = External { map };

    let output = vm.async_call(["main"], (external,)).await?;
    let output = String::from_value(output)?;
    println!("{:?}", output);
    assert_eq!(output, "foo, bar");
    Ok(())
}
