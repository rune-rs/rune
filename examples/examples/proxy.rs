use rune::runtime::{Mut, Ref};
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Any, Context, Diagnostics, FromValue, Source, Sources, Vm};
use std::sync::Arc;

#[derive(Any, Debug, Default)]
struct MyBytes {
    #[allow(unused)]
    bytes: Vec<u8>,
}

#[derive(FromValue)]
struct Proxy {
    field: Mut<String>,
    my_bytes: Ref<MyBytes>,
}

fn main() -> rune::Result<()> {
    let context = Context::with_default_modules()?;
    let mut sources = Sources::new();

    sources.insert(Source::new(
        "test",
        r#"
        pub fn passthrough(my_bytes) {
            #{field: String::from_str("hello world"), my_bytes}
        }
        "#,
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

    let mut vm = Vm::new(Arc::new(context.runtime()), Arc::new(unit));

    let input = MyBytes {
        bytes: vec![77, 77, 77, 77],
    };
    let output = vm.execute(&["passthrough"], (input,))?.complete()?;
    let mut output = Proxy::from_value(output)?;

    println!("field: {:?}", output.field);
    println!("my_bytes: {:?}", output.my_bytes);
    output.field.clear();
    Ok(())
}
