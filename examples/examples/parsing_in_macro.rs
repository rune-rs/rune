use rune::macros::quote;
use rune::parse::Parser;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{ast, ContextError};
use rune::{Diagnostics, Module, Vm};

use std::sync::Arc;

pub fn main() -> rune::support::Result<()> {
    let m = module()?;

    let mut context = rune_modules::default_context()?;
    context.install(m)?;
    let runtime = Arc::new(context.runtime()?);

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
    let output = vm.execute(["main"], ())?.complete().into_result()?;
    let output: (u32, u32) = rune::from_value(output)?;

    println!("{:?}", output);
    Ok(())
}

fn module() -> Result<Module, ContextError> {
    let mut m = Module::default();

    let string = "1 + 2 + 13 * 3";

    m.macro_(["string_as_code"], move |cx, _| {
        let id = cx.insert_source("string_as_code", string)?;
        let expr = cx.parse_source::<ast::Expr>(id)?;

        Ok(quote!(#expr).into_token_stream(cx)?)
    })?;

    m.macro_(["string_as_code_from_arg"], |cx, stream| {
        let mut p = Parser::from_token_stream(stream, cx.input_span());
        let s = p.parse_all::<ast::LitStr>()?;
        let s = cx.resolve(s)?.try_into_owned()?;
        let id = cx.insert_source("string_as_code_from_arg", &s)?;
        let expr = cx.parse_source::<ast::Expr>(id)?;

        Ok(quote!(#expr).into_token_stream(cx)?)
    })?;

    Ok(m)
}
