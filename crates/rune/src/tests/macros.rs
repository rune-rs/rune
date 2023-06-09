use crate as rune;
use crate::ast;
use crate::compile;
use crate::macros::{quote, MacroContext, ToTokens, TokenStream};
use crate::parse::Parser;
use crate::termcolor::{ColorChoice, StandardStream};
use crate::{Context, Diagnostics, Module, Vm, T};

use crate::no_std::prelude::*;
use crate::no_std::sync::Arc;

#[test]
fn test_concat_idents() -> rune::Result<()> {
    #[rune::macro_]
    fn concat_idents(
        ctx: &mut MacroContext<'_, '_>,
        input: &TokenStream,
    ) -> compile::Result<TokenStream> {
        let mut output = String::new();

        let mut p = Parser::from_token_stream(input, ctx.input_span());

        let ident = p.parse::<ast::Ident>()?;
        output.push_str(ctx.resolve(ident)?);

        while p.parse::<Option<T![,]>>()?.is_some() {
            if p.is_eof()? {
                break;
            }

            let ident = p.parse::<ast::Ident>()?;
            output.push_str(ctx.resolve(ident)?);
        }

        p.eof()?;

        let output = ctx.ident(&output);
        Ok(quote!(#output).into_token_stream(ctx))
    }

    let mut m = Module::new();
    m.macro_meta(concat_idents)?;

    let mut context = Context::new();
    context.install(m)?;

    let runtime = Arc::new(context.runtime());

    let mut sources = rune::sources! {
        entry => {
            fn function() {
                42
            }

            pub fn main() {
                let foobar = function();
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

#[test]
fn test_rename() -> rune::Result<()> {
    #[rune::attribute_macro]
    fn rename(
        ctx: &mut MacroContext<'_, '_>,
        input: &TokenStream,
        item: &TokenStream,
    ) -> compile::Result<TokenStream> {
        let mut parser = Parser::from_token_stream(item, ctx.macro_span());
        let mut fun: ast::ItemFn = parser.parse_all()?;

        let mut parser = Parser::from_token_stream(input, ctx.input_span());
        fun.name = parser.parse_all::<ast::EqValue<_>>()?.value;

        let mut tokens = TokenStream::new();
        fun.to_tokens(ctx, &mut tokens);
        Ok(tokens)
    }

    let mut m = Module::new();
    m.macro_meta(rename)?;

    let mut context = Context::new();
    context.install(m)?;

    let runtime = Arc::new(context.runtime());

    let mut sources = rune::sources! {
        entry => {
            #[rename = foobar]
            fn renamed() {
                42
            }

            pub fn main() {
                let foobar = foobar();
                foobar
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
