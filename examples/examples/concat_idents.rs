use rune::ast;
use rune::compile;
use rune::macros::{quote, MacroContext, TokenStream};
use rune::parse::Parser;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Context, Diagnostics, Module, Vm, T};

use std::sync::Arc;

#[rune::macro_]
fn concat_idents(
    cx: &mut MacroContext<'_, '_, '_>,
    stream: &TokenStream,
) -> compile::Result<TokenStream> {
    let mut output = String::new();

    let mut p = Parser::from_token_stream(stream, cx.input_span());

    let ident = p.parse::<ast::Ident>()?;
    output.push_str(cx.resolve(ident)?);

    while p.parse::<Option<T![,]>>()?.is_some() {
        if p.is_eof()? {
            break;
        }

        let ident = p.parse::<ast::Ident>()?;
        output.push_str(cx.resolve(ident)?);
    }

    p.eof()?;

    let output = cx.ident(&output)?;
    Ok(quote!(#output).into_token_stream(cx)?)
}

fn main() -> rune::support::Result<()> {
    let mut m = Module::new();
    m.macro_meta(concat_idents)?;

    let mut context = Context::new();
    context.install(m)?;

    let runtime = Arc::new(context.runtime()?);

    let mut sources = rune::sources! {
        entry => {
            pub fn main() {
                let foobar = 42;
                concat_idents!(foo, bar)
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
    let unit = Arc::new(unit);

    let mut vm = Vm::new(runtime, unit);
    let value = vm.call(["main"], ())?;
    let value: u32 = rune::from_value(value)?;

    assert_eq!(value, 42);
    Ok(())
}
