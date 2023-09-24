use rune::runtime::Vm;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Any, ContextError, Diagnostics, Module};

use std::sync::Arc;

#[derive(Default, Debug, Any, PartialEq, Eq)]
struct Request {
    #[rune(get, set)]
    url: String,
}

#[derive(Default, Debug, Any, PartialEq, Eq)]
#[rune(constructor)]
struct Response {
    #[rune(get, set)]
    status_code: usize,
    #[rune(get, set)]
    body: String,
}

fn main() -> rune::support::Result<()> {
    let m = module()?;

    let mut context = rune_modules::default_context()?;
    context.install(m)?;
    let runtime = Arc::new(context.runtime()?);

    let mut sources = rune::sources! {
        entry => {
            pub fn main(req) {
                let rsp = match req.url {
                    "/" => Response {
                        status_code: 200,
                        body: "ok",
                    },
                    _ => Response {
                        status_code: 400,
                        body: "not found",
                    }
                };

                rsp
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

    let output = vm.call(["main"], (Request { url: "/".into() },))?;
    let output: Response = rune::from_value(output)?;
    println!("{:?}", output);

    let output = vm.call(
        ["main"],
        (Request {
            url: "/account".into(),
        },),
    )?;
    let output: Response = rune::from_value(output)?;
    println!("{:?}", output);

    Ok(())
}

pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new();
    module.ty::<Request>()?;
    module.ty::<Response>()?;
    Ok(module)
}
