//! `std::experiments` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = {version = "0.6.16", features = ["io"]}
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! # fn main() -> runestick::Result<()> {
//! let mut context = runestick::Context::with_default_modules()?;
//! context.install(&rune_modules::io::module(true)?)?;
//! # Ok(())
//! # }
//! ```

use rune::ast;
use rune::macros;
use rune::{quote, IrValue, Parser, Quote, Spanned as _, TokenStream, K, T};
use runestick::format_spec;
use runestick::{Span, SpannedError};
use std::collections::{BTreeMap, BTreeSet};

/// Construct the supplemental `std::io` module.
pub fn module(_stdio: bool) -> Result<runestick::Module, runestick::ContextError> {
    let mut module = runestick::Module::new(&["std", "io"]);
    module.macro_(&["println"], println_macro)?;
    Ok(module)
}

/// Implementation for the `println!` macro.
pub(crate) fn println_macro(stream: &TokenStream) -> runestick::Result<TokenStream> {
    let mut p = Parser::from_token_stream(stream);
    let format = p.parse::<ast::Expr>()?;
    let format_span = format.span();
    let format = macros::eval(&format)?;

    let mut pos = Vec::new();
    let mut named = BTreeMap::new();
    let mut unused_named = BTreeMap::new();

    while p.parse::<Option<T![,]>>()?.is_some() {
        if p.is_eof()? {
            break;
        }

        match (p.nth(0)?, p.nth(1)?) {
            (K![ident], K![=]) => {
                let ident = p.parse::<ast::Ident>()?;
                let key = macros::resolve(ident)?;
                p.parse::<T![=]>()?;
                let expr = p.parse::<ast::Expr>()?;
                unused_named.insert(key.clone(), ident.span().join(expr.span()));
                named.insert(key, expr);
            }
            _ => {
                let expr = p.parse::<ast::Expr>()?;

                if !named.is_empty() {
                    return Err(SpannedError::msg(
                        expr.span(),
                        "unnamed positional arguments must come before named ones",
                    )
                    .into());
                }

                pos.push(expr);
            }
        }
    }

    p.eof()?;

    let format = match format {
        IrValue::String(string) => string.take()?,
        _ => return Err(SpannedError::msg(format_span, "format argument must be a string").into()),
    };

    let mut unused_pos = (0..pos.len()).collect();

    let expanded =
        match expand_format_spec(&format, &pos, &named, &mut unused_pos, &mut unused_named) {
            Ok(expanded) => expanded,
            Err(message) => {
                return Err(SpannedError::msg(format_span, message).into());
            }
        };

    if let Some(expr) = unused_pos.into_iter().flat_map(|n| pos.get(n)).next() {
        return Err(SpannedError::msg(expr.span(), "unused positional argument").into());
    }

    if let Some((key, span)) = unused_named.into_iter().next() {
        return Err(SpannedError::msg(span, format!("unused named argument `{}`", key)).into());
    }

    Ok(quote!(std::io::println(#expanded)).into_token_stream())
}

fn expand_format_spec<'a>(
    input: &str,
    pos: &'a [ast::Expr],
    named: &'a BTreeMap<String, ast::Expr>,
    unused_pos: &mut BTreeSet<usize>,
    unused_named: &mut BTreeMap<String, Span>,
) -> Result<Quote<'a>, Box<str>> {
    let mut it = input.chars();

    let mut arg = String::new();
    let mut buf = String::new();
    let mut in_spec = false;

    let mut components = Vec::new();
    let mut count = 0;

    while let Some(c) = it.next() {
        match c {
            '}' => {
                let c = match it.next() {
                    Some(c) => c,
                    None => return Err("misplaced placeholder '}'".into()),
                };

                match c {
                    '}' => {
                        buf.push('}');
                    }
                    _ => {
                        return Err(
                            "unsupported close `}`, if you meant to escape this use `}}`".into(),
                        );
                    }
                }
            }
            '{' => {
                let mut ty = None;

                loop {
                    let c = match it.next() {
                        Some(c) => c,
                        None => return Err("open placeholder '{', expected `}`".into()),
                    };

                    match c {
                        '{' if !in_spec => {
                            buf.push('{');
                            break;
                        }
                        ':' => {
                            in_spec = true;
                        }
                        '?' if in_spec => {
                            if ty.is_some() {
                                return Err("only one type specifier is supported".into());
                            }

                            ty = Some(format_spec::Type::Debug)
                        }
                        '}' => {
                            if !buf.is_empty() {
                                components.push(C::Literal(buf.clone().into_boxed_str()));
                                buf.clear();
                            }

                            let expr = if !arg.is_empty() {
                                if let Ok(n) = str::parse::<usize>(&arg) {
                                    let expr = match pos.get(n) {
                                        Some(expr) => expr,
                                        None => {
                                            return Err(format!(
                                                "missing positional argument #{}",
                                                n
                                            )
                                            .into())
                                        }
                                    };

                                    unused_pos.remove(&n);
                                    expr
                                } else {
                                    let expr = match named.get(&arg) {
                                        Some(expr) => expr,
                                        None => {
                                            return Err(
                                                format!("missing named argument `{}`", arg).into()
                                            )
                                        }
                                    };

                                    unused_named.remove(&arg);
                                    expr
                                }
                            } else {
                                let expr = match pos.get(count) {
                                    Some(expr) => expr,
                                    None => {
                                        return Err(format!(
                                            "missing positional argument #{}",
                                            count
                                        )
                                        .into())
                                    }
                                };

                                unused_pos.remove(&count);
                                count += 1;
                                expr
                            };

                            components.push(C::Expr { expr, ty });

                            in_spec = false;
                            arg.clear();
                            break;
                        }
                        c if !in_spec => {
                            arg.push(c);
                        }
                        _ => {
                            return Err("unsupported character in spec".into());
                        }
                    }
                }
            }
            o => {
                buf.push(o);
            }
        }
    }

    if !buf.is_empty() {
        components.push(C::Literal(buf.clone().into_boxed_str()));
        buf.clear();
    }

    if components.is_empty() {
        return Ok(quote!(""));
    }

    let mut args = Vec::<Quote<'static>>::new();

    for c in components {
        match c {
            C::Literal(literal) => {
                let lit = ast::Lit::new(&*literal);
                args.push(quote!(#lit));
            }
            C::Expr { expr, ty, .. } => {
                if let Some(ty) = ty {
                    let ty = ast::Ident::new(&ty.to_string());
                    args.push(quote!(
                        #[builtin]
                        format_spec!(#expr, type = #ty)
                    ));
                } else {
                    args.push(quote!(#expr));
                }
            }
        }
    }

    return Ok(quote! {
        #[builtin] template!(#(&args),*)
    });

    enum C<'a> {
        Literal(Box<str>),
        Expr {
            expr: &'a ast::Expr,
            ty: Option<format_spec::Type>,
        },
    }
}
