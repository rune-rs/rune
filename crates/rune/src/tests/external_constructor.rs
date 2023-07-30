prelude!();

use std::sync::Arc;

use rune::{from_value, prepare, sources};
use rune::{Any, Context, ContextError, Module, Vm};

/// Tests pattern matching and constructing over an external variant from within
/// Rune.
#[test]
fn construct_enum() {
    #[derive(Debug, Any, PartialEq, Eq)]
    enum Enum {
        #[rune(constructor)]
        First(#[rune(get)] u32),
        #[rune(constructor)]
        Second(#[rune(get)] u32),
        #[rune(constructor)]
        Third,
        Fourth {
            #[rune(get)]
            a: u32,
            #[rune(get)]
            b: u32,
        },
        #[rune(constructor)]
        Output(#[rune(get)] u32),
    }

    fn make_module() -> Result<Module, ContextError> {
        let mut module = Module::new();
        module.ty::<Enum>()?;
        Ok(module)
    }

    let m = make_module().expect("Module should be buildable");

    let mut context = Context::new();
    context.install(m).expect("Context should build");
    let runtime = Arc::new(context.runtime());

    let mut sources = sources! {
        entry => {
            pub fn main(external) {
                match external {
                    Enum::First(value) => Enum::Output(value * 1),
                    Enum::Second(value) => Enum::Output(value * 2),
                    Enum::Third => Enum::Output(3),
                    Enum::Fourth { a, b } => Enum::Output((a * b) * 4),
                    _ => 0,
                }
            }
        }
    };

    let unit = prepare(&mut sources)
        .with_context(&context)
        .build()
        .expect("Unit should build");

    let mut vm = Vm::new(runtime, Arc::new(unit));

    let output = vm.call(["main"], (Enum::First(42),)).unwrap();
    let output: Enum = from_value(output).unwrap();
    assert_eq!(output, Enum::Output(42));

    let output = vm.call(["main"], (Enum::Second(43),)).unwrap();
    let output: Enum = from_value(output).unwrap();
    assert_eq!(output, Enum::Output(43 * 2));

    let output = vm.call(["main"], (Enum::Third,)).unwrap();
    let output: Enum = from_value(output).unwrap();
    assert_eq!(output, Enum::Output(3));

    let output = vm.call(["main"], (Enum::Fourth { a: 2, b: 3 },)).unwrap();
    let output: Enum = from_value(output).unwrap();
    assert_eq!(output, Enum::Output(2 * 3 * 4));
}

/// Tests pattern matching and constructing over an external struct from within
/// Rune.
#[test]
fn construct_struct() {
    #[derive(Debug, Any, PartialEq, Eq)]
    #[rune(constructor)]
    struct Request {
        #[rune(get)]
        url: String,
    }

    #[derive(Debug, Any, PartialEq, Eq)]
    #[rune(constructor)]
    struct Response {
        #[rune(get, set)]
        status_code: u32,
        #[rune(get, set)]
        body: String,
        #[rune(get, set)]
        content_type: String,
        #[rune(get, set)]
        content_length: u32,
    }

    fn make_module() -> Result<Module, ContextError> {
        let mut module = Module::new();
        module.ty::<Request>()?;
        module.ty::<Response>()?;
        Ok(module)
    }

    let m = make_module().expect("Module should be buildable");

    let mut context = Context::new();
    context.install(m).expect("Context should build");
    let runtime = Arc::new(context.runtime());

    let mut sources = sources! {
        entry => {
            pub fn main(req) {
                let content_type = "text/plain";

                let rsp = match req.url {
                    "/" => Response {
                        status_code: 200,
                        body: "ok",
                        content_type,
                        content_length: 2,
                    },
                    "/account" => Response {
                        content_type,
                        content_length: 12,
                        body: "unauthorized",
                        status_code: 401,
                    },
                    _ => Response {
                        body: "not found",
                        status_code: 404,
                        content_length: 9,
                        content_type,
                    }
                };

                rsp
            }
        }
    };

    let unit = prepare(&mut sources)
        .with_context(&context)
        .build()
        .expect("Unit should build");

    let mut vm = Vm::new(runtime, Arc::new(unit));

    for (req, rsp) in vec![
        (
            Request { url: "/".into() },
            Response {
                status_code: 200,
                body: "ok".into(),
                content_type: "text/plain".into(),
                content_length: 2,
            },
        ),
        (
            Request {
                url: "/account".into(),
            },
            Response {
                status_code: 401,
                body: "unauthorized".into(),
                content_type: "text/plain".into(),
                content_length: 12,
            },
        ),
        (
            Request {
                url: "/cart".into(),
            },
            Response {
                status_code: 404,
                body: "not found".into(),
                content_type: "text/plain".into(),
                content_length: 9,
            },
        ),
    ] {
        let output = vm.call(["main"], (req,)).unwrap();
        let output: Response = from_value(output).unwrap();
        assert_eq!(output.status_code, rsp.status_code);
        assert_eq!(output.body, rsp.body);
    }
}
