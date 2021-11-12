use rune::ast;
use rune::{Context, Diagnostics, FromValue, Module, Options, Parser, Source, Sources, Vm};
use std::sync::Arc;

pub fn main() -> rune::Result<()> {
    let mut m = Module::default();

    let string = "1 + 2 + 13 * 3";

    m.macro_(&["string_as_code"], move |ctx, _| {
        let id = ctx.insert_source("string_as_code", string);
        let expr = ctx.parse_source::<ast::Expr>(id)?;

        Ok(rune::quote!(#expr).into_token_stream(ctx))
    })?;

    m.macro_(&["string_as_code_from_arg"], |ctx, stream| {
        let mut p = Parser::from_token_stream(stream, ctx.stream_span());
        let s = p.parse_all::<ast::LitStr>()?;
        let s = ctx.resolve(s)?.into_owned();
        let id = ctx.insert_source("string_as_code_from_arg", &s);
        let expr = ctx.parse_source::<ast::Expr>(id)?;

        Ok(rune::quote!(#expr).into_token_stream(ctx))
    })?;

    let mut context = Context::with_default_modules()?;
    context.install(&m)?;

    let mut sources = Sources::new();

    sources.insert(Source::new(
        "test",
        r#"
        pub fn main() {
            let a = string_as_code!();
            let b = string_as_code_from_arg!("1 + 2 + 13 * 3");
            (a, b)
        }
        "#,
    ));

    let mut diagnostics = Diagnostics::new();

    let unit = rune::load_sources(
        &context,
        &Options::default(),
        &mut sources,
        &mut diagnostics,
    )?;

    let mut vm = Vm::new(Arc::new(context.runtime()), Arc::new(unit));
    let output = vm.execute(&["main"], ())?.complete()?;
    let output = <(u32, u32)>::from_value(output)?;

    assert_eq!(output, (42, 42));
    Ok(())
}
