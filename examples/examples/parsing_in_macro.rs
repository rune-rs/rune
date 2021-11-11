use rune::{ast, macros, Diagnostics, MacroContext, Options, Parser, Sources};
use runestick::{Context, FromValue, Module, Source, Vm};
use std::sync::Arc;

pub fn main() -> runestick::Result<()> {
    let mut m = Module::default();

    let string = "1 + 2 + 13 * 3";

    m.macro_(
        &["string_as_code"],
        move |ctx: &mut MacroContext<'_>, _: &macros::TokenStream| {
            let expr = ctx.parse_all::<ast::Expr>(&string)?;
            Ok(rune::quote!(#expr).into_token_stream(ctx))
        },
    )?;

    m.macro_(
        &["string_as_code_from_arg"],
        |ctx: &mut MacroContext<'_>, stream| {
            let mut p = Parser::from_token_stream(stream, ctx.stream_span());
            let s = p.parse_all::<ast::LitStr>()?;
            let s = ctx.resolve(s)?;
            let expr = ctx.parse_all::<ast::Expr>(&s)?;
            Ok(rune::quote!(#expr).into_token_stream(ctx))
        },
    )?;

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
