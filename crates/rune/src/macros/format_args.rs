use crate::ast;
use crate::ir::IrValue;
use crate::macros;
use crate::macros::Quote;
use crate::parsing::{Parse, ParseError, Parser, Peek};
use crate::quote;
use crate::Spanned as _;
use runestick::format_spec;
use runestick::{Span, SpannedError};
use std::collections::{BTreeMap, BTreeSet};

// NB: needed for quote macro.
use crate as rune;

/// A format specification: A format string followed by arguments to be
/// formatted in accordance with that string.
///
/// This type can only be parsed inside of a macro context since it performs
/// constant evaluation.
pub struct FormatArgs {
    /// Format argument.
    format: ast::Expr,
    /// Positional arguments.
    pos: Vec<ast::Expr>,
    /// Named arguments.
    named: BTreeMap<Box<str>, (ast::Ident, T![=], ast::Expr)>,
}

impl FormatArgs {
    /// Expand the format specification.
    ///
    /// # Panics
    ///
    /// Panics if called outside of a macro context.
    pub fn expand(&self) -> Result<Quote<'_>, SpannedError> {
        let format = macros::eval(&self.format)
            .map_err(|error| SpannedError::new(error.span(), error.into_kind()))?;

        let format = match format {
            IrValue::String(string) => string
                .take()
                .map_err(|error| SpannedError::new(self.format.span(), error))?,
            _ => {
                return Err(SpannedError::msg(
                    self.format.span(),
                    "format argument must be a string",
                )
                .into())
            }
        };

        let mut unused_pos = (0..self.pos.len()).collect::<BTreeSet<_>>();
        let mut unused_named = self
            .named
            .iter()
            .map(|(key, n)| (key.clone(), n.0.span().join(n.1.span())))
            .collect::<BTreeMap<_, _>>();

        let expanded = match expand_format_spec(
            &format,
            &self.pos,
            &self.named,
            &mut unused_pos,
            &mut unused_named,
        ) {
            Ok(expanded) => expanded,
            Err(message) => {
                return Err(SpannedError::msg(self.format.span(), message).into());
            }
        };

        if let Some(expr) = unused_pos.into_iter().flat_map(|n| self.pos.get(n)).next() {
            return Err(SpannedError::msg(expr.span(), "unused positional argument").into());
        }

        if let Some((key, span)) = unused_named.into_iter().next() {
            return Err(SpannedError::msg(span, format!("unused named argument `{}`", key)).into());
        }

        Ok(expanded)
    }
}

impl Parse for FormatArgs {
    /// Parse format arguments inside of a macro.
    fn parse(p: &mut Parser<'_>) -> Result<Self, ParseError> {
        if p.is_eof()? {
            return Err(ParseError::custom(p.last_span(), "expected format specifier").into());
        }

        let format = p.parse::<ast::Expr>()?;

        let mut pos = Vec::new();
        let mut named = BTreeMap::new();

        while p.parse::<Option<T![,]>>()?.is_some() {
            if p.is_eof()? {
                break;
            }

            match (p.nth(0)?, p.nth(1)?) {
                (K![ident], K![=]) => {
                    let ident = p.parse::<ast::Ident>()?;
                    let key = macros::resolve(ident)?;
                    let eq_token = p.parse::<T![=]>()?;
                    let expr = p.parse::<ast::Expr>()?;
                    named.insert(key.into(), (ident, eq_token, expr));
                }
                _ => {
                    let expr = p.parse::<ast::Expr>()?;

                    if !named.is_empty() {
                        return Err(ParseError::custom(
                            expr.span(),
                            "unnamed positional arguments must come before named ones",
                        ));
                    }

                    pos.push(expr);
                }
            }
        }

        Ok(Self { format, pos, named })
    }
}

impl Peek for FormatArgs {
    fn peek(p: &mut crate::Peeker<'_>) -> bool {
        !p.is_eof()
    }
}

fn expand_format_spec<'a>(
    input: &str,
    pos: &'a [ast::Expr],
    named: &'a BTreeMap<Box<str>, (ast::Ident, T![=], ast::Expr)>,
    unused_pos: &mut BTreeSet<usize>,
    unused_named: &mut BTreeMap<Box<str>, Span>,
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
                                    let expr = match named.get(arg.as_str()) {
                                        Some((_, _, expr)) => expr,
                                        None => {
                                            return Err(
                                                format!("missing named argument `{}`", arg).into()
                                            )
                                        }
                                    };

                                    unused_named.remove(arg.as_str());
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
