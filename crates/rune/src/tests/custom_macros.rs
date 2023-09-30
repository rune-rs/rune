prelude!();

use std::sync::Arc;

use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use macros::quote;
use parse::Parser;

#[test]
fn test_parse_in_macro() -> Result<()> {
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

    let mut context = Context::with_default_modules()?;
    context.install(m)?;

    let mut sources = sources! {
        entry => {
            pub fn main() {
                let a = string_as_code!();
                let b = string_as_code_from_arg!("1 + 2 + 13 * 3");
                (a, b)
            }
        }
    };

    let unit = prepare(&mut sources).with_context(&context).build()?;

    let mut vm = Vm::new(Arc::new(context.runtime()?), Arc::new(unit));
    let output = vm.call(["main"], ())?;
    let output: (u32, u32) = from_value(output)?;

    assert_eq!(output, (42, 42));
    Ok(())
}

#[test]
fn conflicting_attribute_function() -> Result<()> {
    let mut m = Module::default();

    m.macro_(["conflicting"], move |cx, _| {
        Ok(quote!(21).into_token_stream(cx)?)
    })?;

    m.attribute_macro(["conflicting"], |cx, _, _| {
        let stream = quote!(
            fn hello() {
                21
            }
        );

        Ok(stream.into_token_stream(cx)?)
    })?;

    let mut context = Context::with_default_modules()?;
    context.install(m)?;

    let mut sources = sources! {
        entry => {
            pub fn main() {
                hello() + conflicting!()
            }

            #[conflicting]
            fn hi() {}
        }
    };

    let unit = prepare(&mut sources).with_context(&context).build()?;

    let mut vm = Vm::new(Arc::new(context.runtime()?), Arc::new(unit));
    let output = vm.call(["main"], ())?;
    let output: u32 = from_value(output)?;

    assert_eq!(output, 42);
    Ok(())
}

#[test]
fn attribute_imports_builtin() -> Result<()> {
    let mut m = Module::with_crate("abc")?;

    m.attribute_macro(["before_use"], |cx, _, _| {
        let stream = quote!(
            fn before() {
                21
            }
        );

        Ok(stream.into_token_stream(cx)?)
    })?;

    m.attribute_macro(["after_use"], |cx, _, _| {
        let stream = quote!(
            fn after() {
                21
            }
        );

        Ok(stream.into_token_stream(cx)?)
    })?;

    let mut context = Context::with_default_modules()?;
    context.install(m)?;

    let mut sources = sources! {
        entry => {
            #[doc = "Doc comment"]
            #[test]
            pub fn main() {
                before() + after()
            }

            #[before_use]
            fn hi() {}

            use ::abc::{ before_use, after_use };

            #[after_use]
            fn ho() {}

        }
    };

    let diagnostics = &mut Diagnostics::new();

    let result = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(diagnostics)
        .build();

    if !diagnostics.is_empty() {
        diagnostics.emit(&mut StandardStream::stdout(ColorChoice::Auto), &sources)?;
    }

    let unit = result?;

    let mut vm = Vm::new(Arc::new(context.runtime()?), Arc::new(unit));
    let output = vm.call(["main"], ())?;
    let output: u32 = from_value(output)?;

    assert_eq!(output, 42);
    Ok(())
}
