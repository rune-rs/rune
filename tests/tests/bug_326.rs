use rune::{ContextError, Module, Source, Sources, Vm};
use std::sync::Arc;

/// Cannot call instance functions on template literals.
/// https://github.com/rune-rs/rune/issues/326
#[test]
fn bug_326() -> rune::Result<()> {
    let mut context = rune_modules::default_context()?;
    context.install(&trim_module()?)?;

    let runtime = Arc::new(context.runtime());

    let mut sources = Sources::new();
    sources.insert(Source::new(
        "script",
        r#"
        pub fn test_multiline_template() {
            let age = 35;

            let template_runtime_failure =
                `Hello, I am ${age} years old.
                  How old are you?`.trim_indent();

            println(template_runtime_failure);
        }
        "#,
    ));

    let result = rune::prepare(&mut sources)
        .with_context(&context)
        .build();

    let unit = result?;
    let mut vm = Vm::new(runtime, Arc::new(unit));

    vm.call(&["test_multiline_template"], ())?;
    Ok(())
}

fn trim_module() -> Result<Module, ContextError> {
    let mut m = Module::with_item(&["mymodule"]);
    m.inst_fn("trim_indent", trim_indent)?;
    Ok(m)
}

fn trim_indent(string: String) -> String {
    string
        .lines()
        .map(|s| s.trim_start())
        .collect::<Vec<&str>>()
        .join("\n")
}