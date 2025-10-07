use crate as rune;
use crate::ast;
use crate::compile;
use crate::macros::{quote, MacroContext, ToTokens, TokenStream};
use crate::parse::Parser;
use crate::termcolor::{ColorChoice, StandardStream};
use crate::{Context, Diagnostics, Module, Vm};

use crate::tests::prelude::*;

mod stringy_math;

#[test]
fn test_concat_idents() -> rune::support::Result<()> {
    let sources = rune::sources! {
        entry => {
            use ::test::macros::concat_idents;

            fn function() {
                42
            }

            pub fn main() {
                let foobar = concat_idents!(fu, nction)();
                concat_idents!(foo, bar)
            }
        }
    };

    let mut vm = compile(sources)?;
    let value = vm.call(["main"], ())?;
    let value: u32 = rune::from_value(value)?;

    assert_eq!(value, 42);
    Ok(())
}

#[test]
fn test_rename() -> rune::support::Result<()> {
    let mut vm = compile(rune::sources! {
        entry => {
            use ::test::macros::rename;

            #[rename = foobar]
            fn renamed() {
                42
            }

            pub fn main() {
                let foobar = foobar();
                foobar
            }
        }
    })?;

    let value = vm.call(["main"], ())?;
    let value: u32 = rune::from_value(value)?;

    assert_eq!(value, 42);
    Ok(())
}

#[test]
fn test_make_function() -> rune::support::Result<()> {
    let mut vm = compile(rune::sources! {
        entry => {
            pub fn main() {
                root_fn()
            }

            make_function!(root_fn => { 42 });

            // NB: we put the import in the bottom to test that import resolution isn't order-dependent.
            use ::test::macros::make_function;
        }
    })?;

    let value = vm.call(["main"], ())?;
    let value: u32 = rune::from_value(value)?;

    assert_eq!(value, 42);
    Ok(())
}

#[test]
fn test_stringy_math() -> rune::support::Result<()> {
    let mut vm = compile(rune::sources! {
        entry => {
            use ::test::macros::stringy_math;

            pub fn main() {
                stringy_math!(add 10 sub 2 div 3 mul 100)
            }
        }
    })?;

    let value = vm.call(["main"], ())?;
    let value: u32 = rune::from_value(value)?;

    assert_eq!(value, 200);
    Ok(())
}

#[rune::macro_]
fn concat_idents(
    cx: &mut MacroContext<'_, '_, '_>,
    input: &TokenStream,
) -> compile::Result<TokenStream> {
    let mut output = String::new();

    let mut p = Parser::from_token_stream(input, cx.input_span());

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

#[rune::attribute_macro]
fn rename(
    cx: &mut MacroContext<'_, '_, '_>,
    input: &TokenStream,
    item: &TokenStream,
) -> compile::Result<TokenStream> {
    let mut parser = Parser::from_token_stream(item, cx.macro_span());
    let mut fun: ast::ItemFn = parser.parse_all()?;

    let mut parser = Parser::from_token_stream(input, cx.input_span());
    fun.name = parser.parse_all::<ast::EqValue<_>>()?.value;

    let mut tokens = TokenStream::new();
    fun.to_tokens(cx, &mut tokens)?;
    Ok(tokens)
}

/// Implementation for the `passthrough!` macro.
#[rune::macro_]
fn passthrough(
    _: &mut MacroContext<'_, '_, '_>,
    stream: &TokenStream,
) -> compile::Result<TokenStream> {
    Ok(stream.try_clone()?)
}

/// Implementation for the `make_function!` macro.
#[rune::macro_]
fn make_function(
    cx: &mut MacroContext<'_, '_, '_>,
    stream: &TokenStream,
) -> compile::Result<TokenStream> {
    let mut parser = Parser::from_token_stream(stream, cx.input_span());

    let ident = parser.parse::<ast::Ident>()?;
    let _ = parser.parse::<T![=>]>()?;
    let output = parser.parse::<ast::ExprBlock>()?;
    parser.eof()?;

    Ok(quote!(fn #ident() { #output }).into_token_stream(cx)?)
}

fn compile(mut sources: Sources) -> Result<Vm> {
    let mut m = Module::with_crate_item("test", ["macros"])?;

    m.macro_meta(concat_idents)?;
    m.macro_meta(rename)?;
    m.macro_meta(stringy_math::stringy_math)?;
    m.macro_meta(passthrough)?;
    m.macro_meta(make_function)?;

    let mut context = Context::with_default_modules()?;
    context.install(m)?;

    let runtime = Arc::try_new(context.runtime()?)?;
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
    let unit = Arc::try_new(unit)?;
    Ok(Vm::new(runtime, unit))
}
