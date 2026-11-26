use crate as rune;
use crate::support::Result;
use crate::sync::Arc;
use crate::termcolor::{ColorChoice, StandardStream};
use crate::{Context, ContextError, Diagnostics, Hash, Module, Options, Source, Sources, Unit, Vm};

#[rune::function]
fn calculate(value: i64) -> i64 {
    value + 10
}

pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new();
    module.function_meta(calculate)?;
    Ok(module)
}

#[test]
fn simple_script() -> Result<()> {
    let context = context()?;
    let runtime = Arc::try_new(context.runtime()?)?;

    let unit = compile(&context, "calculate(value) / 2")?;
    let mut vm = Vm::new(runtime, unit);

    let output = vm.call(Hash::EMPTY, (5,))?;
    let output: i64 = crate::from_value(output)?;
    assert_eq!(output, 7);
    Ok(())
}

#[test]
fn shebang_script() -> Result<()> {
    let context = context()?;
    let runtime = Arc::try_new(context.runtime()?)?;

    let unit = compile(
        &context,
        "#!/usr/bin/env -S rune run\ncalculate(value) / 2\n",
    )?;
    let mut vm = Vm::new(runtime, unit);

    let output = vm.call(Hash::EMPTY, (5,))?;
    let output: i64 = crate::from_value(output)?;
    assert_eq!(output, 7);
    Ok(())
}

#[test]
fn shebang_script_with_main() -> Result<()> {
    let context = Context::with_default_modules()?;
    let runtime = Arc::try_new(context.runtime()?)?;

    let mut sources = Sources::new();
    sources.insert(Source::memory(
        "#!/usr/bin/env -S rune run\n\npub fn main() { 42 }\n",
    )?)?;

    let options = Options {
        script: true,
        ..Options::default()
    };

    let unit = crate::prepare(&mut sources)
        .with_context(&context)
        .with_options(&options)
        .build()?;

    let mut vm = Vm::new(runtime, Arc::try_new(unit)?);
    let output: i64 = crate::from_value(vm.call(["main"], ())?)?;
    assert_eq!(output, 42);
    Ok(())
}

#[test]
fn shebang_module() -> Result<()> {
    for (source, expected) in [
        ("#!/usr/bin/env -S rune run\n\npub fn main() { 42 }", 42i64),
        (
            "#!/usr/bin/env rune\n#![doc = \"module doc\"]\npub fn main() { 7 }",
            7i64,
        ),
        ("#![doc = \"module doc\"]\npub fn main() { 9 }", 9i64),
    ] {
        let context = Context::with_default_modules()?;
        let runtime = Arc::try_new(context.runtime()?)?;

        let mut sources = Sources::new();
        sources.insert(Source::memory(source)?)?;

        let unit = crate::prepare(&mut sources)
            .with_context(&context)
            .build()?;

        let mut vm = Vm::new(runtime, Arc::try_new(unit)?);
        let output: i64 = crate::from_value(vm.call(["main"], ())?)?;
        assert_eq!(output, expected);
    }

    Ok(())
}

#[test]
fn shebang_only_script() -> Result<()> {
    let context = context()?;
    let runtime = Arc::try_new(context.runtime()?)?;

    let unit = compile(&context, "#!/usr/bin/env -S rune run\n")?;
    let mut vm = Vm::new(runtime, unit);

    let output: () = crate::from_value(vm.call(Hash::EMPTY, (5,))?)?;
    assert_eq!(output, ());
    Ok(())
}

fn context() -> Result<Arc<Context>, ContextError> {
    let m = module()?;
    let mut context = Context::with_default_modules()?;
    context.install(m)?;
    Ok(Arc::try_new(context)?)
}

fn compile(context: &Context, script: &str) -> Result<Arc<Unit>> {
    let mut sources = Sources::new();
    sources.insert(Source::memory(script)?)?;

    let mut diagnostics = Diagnostics::new();
    let mut options = Options::from_default_env()?;
    options.script(true);

    let result = crate::prepare(&mut sources)
        .with_args(["value"])?
        .with_options(&options)
        .with_context(context)
        .with_diagnostics(&mut diagnostics)
        .build();

    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        diagnostics.emit(&mut writer, &sources)?;
    }

    let unit = result?;
    let unit = Arc::try_new(unit)?;
    Ok(unit)
}
