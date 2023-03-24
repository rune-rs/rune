use rune::macros::quote;
use rune::parse::Parser;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{ast, ContextError};
use rune::{Diagnostics, FromValue, Module, Vm};
use std::sync::Arc;

pub fn main() -> rune::Result<()> {
    let m = module()?;

    let mut context = rune_modules::default_context()?;
    context.install(m)?;
    let runtime = Arc::new(context.runtime());

    let mut sources = rune::sources! {
        entry => {
            pub fn main() {
                let a = string_as_code!();
                let b = string_as_code_from_arg!("1 + 2 + 13 * 3");
                (a, b)
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
    let output = vm.execute(["main"], ())?.complete()?;
    let output = <(u32, u32)>::from_value(output)?;

    println!("{:?}", output);
    Ok(())
}

fn module() -> Result<Module, ContextError> {
    let mut m = Module::default();

    let string = "1 + 2 + 13 * 3";

    m.macro_(["string_as_code"], move |ctx, _| {
        let id = ctx.insert_source("string_as_code", string);
        let expr = ctx.parse_source::<ast::Expr>(id)?;

        Ok(quote!(#expr).into_token_stream(ctx))
    })?;

    m.macro_(["string_as_code_from_arg"], |ctx, stream| {
        let mut p = Parser::from_token_stream(stream, ctx.stream_span());
        let s = p.parse_all::<ast::LitStr>()?;
        let s = ctx.resolve(s)?.into_owned();
        let id = ctx.insert_source("string_as_code_from_arg", &s);
        let expr = ctx.parse_source::<ast::Expr>(id)?;

        Ok(quote!(#expr).into_token_stream(ctx))
    })?;

    Ok(m)
}
