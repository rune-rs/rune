use rune::{termcolor, Runtime};
use runestick::{Context, FromValue, Hash, Item, Module};
use std::io::Write as _;

fn divide_by_three(value: i64) -> i64 {
    value / 3
}

#[tokio::main]
async fn main() -> runestick::Result<()> {
    let mut my_module = Module::new(&["mymodule"]);
    my_module.inst_fn("divide_by_three", divide_by_three)?;

    let mut context = Context::with_default_modules()?;
    context.install(&my_module)?;

    let mut runtime = Runtime::with_context(context);

    let result = runtime.load_source(
        String::from("test"),
        String::from(
            r#"
            fn call_instance_fn(number) {
                number.divide_by_three()
            }
            "#,
        ),
    );

    let file_id = match result {
        Ok(file_id) => file_id,
        Err(e) => {
            let mut writer = termcolor::StandardStream::stderr(termcolor::ColorChoice::Never);
            writeln!(writer, "failed to load source: {}", e)?;
            runtime.emit_diagnostics(&mut writer)?;
            return Ok(());
        }
    };

    let vm = runtime.unit_vm(file_id).unwrap();

    let output = vm
        .call_function(Hash::type_hash(Item::of(&["call_instance_fn"])), (33i64,))?
        .run_to_completion()
        .await?;

    let output = i64::from_value(output)?;
    println!("output: {}", output);
    Ok(())
}
