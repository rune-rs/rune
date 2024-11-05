use core::str;

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::{self, BTreeMap, BTreeSet, Box, HashMap, String, Vec};
use crate::ast::{self, Span, Spanned};
use crate::compile::{self, WithSpan};
use crate::macros::{quote, MacroContext, Quote, ToTokens, TokenStream};
use crate::parse::{Parse, Parser, Peek, Peeker};
use crate::runtime::{format, Inline};

/// A format specification: A format string followed by arguments to be
/// formatted in accordance with that string.
///
/// This type can only be parsed inside of a macro context since it performs
/// constant evaluation.
pub struct FormatArgs {
    /// Format argument.
    format: ast::Expr,
    /// Format arguments.
    args: Vec<FormatArg>,
}

impl FormatArgs {
    /// Expand the format specification.
    pub fn expand(&self, cx: &mut MacroContext<'_, '_, '_>) -> compile::Result<Quote<'_>> {
        let format = cx.eval(&self.format)?;

        let mut pos = Vec::new();
        let mut named = HashMap::<Box<str>, _>::new();

        for a in &self.args {
            match a {
                FormatArg::Positional(expr) => {
                    if !named.is_empty() {
                        return Err(compile::Error::msg(
                            expr.span(),
                            "unnamed positional arguments must come before named ones",
                        ));
                    }

                    pos.try_push(expr)?;
                }
                FormatArg::Named(n) => {
                    let name = cx.resolve(n.key)?;
                    named.try_insert(name.try_into()?, n)?;
                }
            }
        }

        let format = format.downcast::<String>().with_span(&self.format)?;

        let mut unused_pos = (0..pos.len()).try_collect::<BTreeSet<_>>()?;
        let mut unused_named = named
            .iter()
            .map(|(key, n)| Ok::<_, alloc::Error>((key.try_clone()?, n.span())))
            .try_collect::<alloc::Result<BTreeMap<_, _>>>()??;

        let result = expand_format_spec(
            cx,
            self.format.span(),
            &format,
            &pos,
            &mut unused_pos,
            &named,
            &mut unused_named,
        );

        let expanded = match result {
            Ok(expanded) => expanded,
            Err(message) => return Err(compile::Error::msg(self.format.span(), message)),
        };

        if let Some(expr) = unused_pos.into_iter().flat_map(|n| pos.get(n)).next() {
            return Err(compile::Error::msg(
                expr.span(),
                "unused positional argument",
            ));
        }

        if let Some((key, span)) = unused_named.into_iter().next() {
            return Err(compile::Error::msg(
                span,
                format!("unused named argument `{}`", key),
            ));
        }

        Ok(expanded)
    }
}

impl Parse for FormatArgs {
    /// Parse format arguments inside of a macro.
    fn parse(p: &mut Parser<'_>) -> compile::Result<Self> {
        if p.is_eof()? {
            return Err(compile::Error::msg(
                p.last_span(),
                "expected format specifier",
            ));
        }

        let format = p.parse::<ast::Expr>()?;

        let mut args = Vec::new();

        while p.parse::<Option<T![,]>>()?.is_some() {
            if p.is_eof()? {
                break;
            }

            args.try_push(p.parse()?)?;
        }

        Ok(Self { format, args })
    }
}

impl Peek for FormatArgs {
    fn peek(p: &mut Peeker<'_>) -> bool {
        !p.is_eof()
    }
}

/// A named format argument.
#[derive(Debug, TryClone, Parse, Spanned)]
pub struct NamedFormatArg {
    /// The key of the named argument.
    pub key: ast::Ident,
    /// The `=` token.
    pub eq_token: T![=],
    /// The value expression.
    pub expr: ast::Expr,
}

/// A single format argument.
#[derive(Debug, TryClone)]
pub enum FormatArg {
    /// A positional argument.
    Positional(ast::Expr),
    /// A named argument.
    Named(NamedFormatArg),
}

impl Parse for FormatArg {
    fn parse(p: &mut Parser<'_>) -> compile::Result<Self> {
        Ok(if let (K![ident], K![=]) = (p.nth(0)?, p.nth(1)?) {
            FormatArg::Named(p.parse()?)
        } else {
            FormatArg::Positional(p.parse()?)
        })
    }
}

fn expand_format_spec<'a>(
    cx: &mut MacroContext<'_, '_, '_>,
    span: Span,
    input: &str,
    pos: &[&'a ast::Expr],
    unused_pos: &mut BTreeSet<usize>,
    named: &HashMap<Box<str>, &'a NamedFormatArg>,
    unused_named: &mut BTreeMap<Box<str>, Span>,
) -> compile::Result<Quote<'a>> {
    let mut iter = Iter::new(input);

    let mut name = String::new();
    let mut width = String::new();
    let mut precision = String::new();

    let mut buf = String::new();
    let mut components = Vec::new();
    let mut count = 0;
    let mut start = Some(0);

    while let Some((at, a, b)) = iter.next() {
        match (a, b) {
            ('}', '}') => {
                if let Some(start) = start.take() {
                    buf.try_push_str(&input[start..at])?;
                }

                buf.try_push('}')?;
                iter.next();
            }
            ('{', '{') => {
                if let Some(start) = start.take() {
                    buf.try_push_str(&input[start..at])?;
                }

                buf.try_push('{')?;
                iter.next();
            }
            ('}', _) => {
                return Err(compile::Error::msg(
                    span,
                    "unsupported close `}`, if you meant to escape this use `}}`",
                ));
            }
            ('{', _) => {
                if let Some(start) = start.take() {
                    buf.try_push_str(&input[start..at])?;
                }

                if !buf.is_empty() {
                    components.try_push(C::Literal(Box::try_from(&buf[..])?))?;
                    buf.clear();
                }

                components.try_push(parse_group(
                    cx,
                    span,
                    &mut iter,
                    &mut count,
                    &mut name,
                    &mut width,
                    &mut precision,
                    pos,
                    unused_pos,
                    named,
                    unused_named,
                )?)?;
            }
            _ => {
                if start.is_none() {
                    start = Some(at);
                }
            }
        }
    }

    if let Some(start) = start.take() {
        buf.try_push_str(&input[start..])?;
    }

    if !buf.is_empty() {
        components.try_push(C::Literal(Box::try_from(&buf[..])?))?;
        buf.clear();
    }

    if components.is_empty() {
        return Ok(quote!(""));
    }

    let mut args = Vec::<Quote<'static>>::new();

    for c in components {
        match c {
            C::Literal(literal) => {
                let lit = cx.lit(literal.as_ref())?;
                args.try_push(quote!(#lit))?;
            }
            C::Format {
                expr,
                fill,
                align,
                width,
                precision,
                flags,
                format_type,
            } => {
                let mut specs = Vec::new();

                let fill = fill
                    .map(|fill| {
                        let fill = cx.lit(fill)?;
                        Ok::<_, alloc::Error>(quote!(fill = #fill))
                    })
                    .transpose()?;

                let width = width
                    .map(|width| {
                        let width = cx.lit(width)?;
                        Ok::<_, alloc::Error>(quote!(width = #width))
                    })
                    .transpose()?;

                let precision = precision
                    .map(|precision| {
                        let precision = cx.lit(precision)?;
                        Ok::<_, alloc::Error>(quote!(precision = #precision))
                    })
                    .transpose()?;

                let align = align
                    .map(|align| {
                        let align = align.try_to_string()?;
                        let align = cx.ident(&align)?;
                        Ok::<_, alloc::Error>(quote!(align = #align))
                    })
                    .transpose()?;

                specs.try_extend(fill)?;
                specs.try_extend(width)?;
                specs.try_extend(precision)?;
                specs.try_extend(align)?;

                if !flags.is_empty() {
                    let flags = cx.lit(flags.into_u32())?;
                    specs.try_push(quote!(flags = #flags))?;
                }

                let format_type = format_type
                    .map(|format_type| {
                        let format_type = format_type.try_to_string()?;
                        let format_type = cx.ident(&format_type)?;
                        Ok::<_, alloc::Error>(quote!(type = #format_type))
                    })
                    .transpose()?;

                specs.try_extend(format_type)?;

                if specs.is_empty() {
                    args.try_push(quote!(#expr))?;
                } else {
                    args.try_push(quote!(
                        #[builtin]
                        format!(#expr, #(specs),*)
                    ))?;
                }
            }
        }
    }

    return Ok(quote! {
        #[builtin] template!(#(args),*)
    });

    enum ExprOrIdent<'a> {
        Expr(&'a ast::Expr),
        Ident(ast::Ident),
    }

    impl ToTokens for ExprOrIdent<'_> {
        fn to_tokens(
            &self,
            cx: &mut MacroContext<'_, '_, '_>,
            stream: &mut TokenStream,
        ) -> alloc::Result<()> {
            match self {
                Self::Expr(expr) => expr.to_tokens(cx, stream),
                Self::Ident(ident) => ident.to_tokens(cx, stream),
            }
        }
    }

    enum C<'a> {
        Literal(Box<str>),
        Format {
            expr: ExprOrIdent<'a>,
            fill: Option<char>,
            align: Option<format::Alignment>,
            width: Option<usize>,
            precision: Option<usize>,
            flags: format::Flags,
            format_type: Option<format::Type>,
        },
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum Mode {
        /// Start of parser.
        Start,
        // Parse alignment.
        FillAllign,
        // '+' or '-' encountered.
        Sign,
        // Alternate '#' encountered.
        Alternate,
        // Sign aware zero pad `0` encountered.
        SignAwareZeroPad,
        // Parse width.
        Width,
        /// We've parsed precision fully already.
        Precision,
        // Type e.g. `?` encountered.
        Type,
        // Final mode.
        End,
    }

    /// Parse a single expansion group.
    fn parse_group<'a>(
        cx: &mut MacroContext<'_, '_, '_>,
        span: Span,
        iter: &mut Iter<'_>,
        count: &mut usize,
        name: &mut String,
        width: &mut String,
        precision: &mut String,
        pos: &[&'a ast::Expr],
        unused_pos: &mut BTreeSet<usize>,
        named: &HashMap<Box<str>, &'a NamedFormatArg>,
        unused_named: &mut BTreeMap<Box<str>, Span>,
    ) -> compile::Result<C<'a>> {
        use num::ToPrimitive as _;

        // Parsed flags.
        let mut flags = format::Flags::default();
        // Parsed fill character.
        let mut fill = None;
        // Parsed alignment.
        let mut align = None;
        // We are expecting to receive precision as a positional parameter.
        let mut input_precision = false;
        // Parsed formatting type.
        let mut format_type = None;

        // Clear re-used temporary buffers.
        name.clear();
        width.clear();
        precision.clear();

        let mut mode = Mode::Start;

        loop {
            let Some((_, a, b)) = iter.current() else {
                return Err(compile::Error::msg(span, "unexpected end of format string"));
            };

            match mode {
                Mode::Start => match a {
                    ':' => {
                        mode = Mode::FillAllign;
                        iter.next();
                    }
                    '}' => {
                        mode = Mode::End;
                    }
                    c => {
                        name.try_push(c)?;
                        iter.next();
                    }
                },
                Mode::FillAllign => {
                    // NB: parse alignment, if present.
                    if matches!(a, '<' | '^' | '>') {
                        align = Some(parse_align(a));
                        iter.next();
                    } else if matches!(b, '<' | '^' | '>') {
                        fill = Some(a);
                        align = Some(parse_align(b));

                        iter.next();
                        iter.next();
                    }

                    mode = Mode::Sign;
                }
                Mode::Sign => {
                    match a {
                        '-' => {
                            flags.set(format::Flag::SignMinus);
                            iter.next();
                        }
                        '+' => {
                            flags.set(format::Flag::SignPlus);
                            iter.next();
                        }
                        _ => (),
                    }

                    mode = Mode::Alternate;
                }
                Mode::Alternate => {
                    if a == '#' {
                        flags.set(format::Flag::Alternate);
                        iter.next();
                    }

                    mode = Mode::SignAwareZeroPad;
                }
                Mode::SignAwareZeroPad => {
                    if a == '0' {
                        flags.set(format::Flag::SignAwareZeroPad);
                        iter.next();
                    }

                    mode = Mode::Width;
                }
                Mode::Width => {
                    match a {
                        '0'..='9' => {
                            width.try_push(a)?;
                            iter.next();
                            continue;
                        }
                        '.' => {
                            mode = Mode::Precision;
                            iter.next();
                            continue;
                        }
                        _ => (),
                    }

                    mode = Mode::Type;
                }
                Mode::Precision => {
                    match a {
                        '*' if precision.is_empty() => {
                            input_precision = true;
                            iter.next();
                        }
                        '0'..='9' => {
                            precision.try_push(a)?;
                            iter.next();
                            continue;
                        }
                        _ => (),
                    }

                    mode = Mode::Type;
                }
                Mode::Type => {
                    match a {
                        '?' => {
                            format_type = Some(format::Type::Debug);
                            iter.next();
                        }
                        'x' => {
                            format_type = Some(format::Type::LowerHex);
                            iter.next();
                        }
                        'X' => {
                            format_type = Some(format::Type::UpperHex);
                            iter.next();
                        }
                        'b' => {
                            format_type = Some(format::Type::Binary);
                            iter.next();
                        }
                        'p' => {
                            format_type = Some(format::Type::Pointer);
                            iter.next();
                        }
                        _ => (),
                    }

                    mode = Mode::End;
                }
                Mode::End => {
                    match a {
                        '}' => (),
                        c => {
                            return Err(compile::Error::msg(
                                span,
                                format!("unsupported char `{}` in spec", c),
                            ));
                        }
                    }

                    iter.next();
                    break;
                }
            }
        }

        let precision = if input_precision {
            let &expr = match pos.get(*count) {
                Some(expr) => expr,
                None => {
                    return Err(compile::Error::msg(
                        span,
                        format!(
                            "missing positional argument #{} \
                            which is required for position parameter",
                            count
                        ),
                    ));
                }
            };

            unused_pos.remove(count);

            let value = cx.eval(expr)?;

            let number = match value.as_inline() {
                Some(Inline::Signed(n)) => n.to_usize(),
                _ => None,
            };

            let precision = if let Some(number) = number {
                number
            } else {
                let span = expr.span();

                return Err(compile::Error::msg(
                    span,
                    format!(
                        "expected position argument #{} \
                        to be a positive number in use as precision, \
                        but got `{}`",
                        count,
                        value.type_info()
                    ),
                ));
            };

            *count += 1;
            Some(precision)
        } else if !precision.is_empty() {
            str::parse::<usize>(precision).ok()
        } else {
            None
        };

        let expr = 'expr: {
            if name.is_empty() {
                let Some(expr) = pos.get(*count) else {
                    return Err(compile::Error::msg(
                        span,
                        format!("missing positional argument #{count}"),
                    ));
                };

                unused_pos.remove(count);
                *count += 1;
                break 'expr ExprOrIdent::Expr(expr);
            };

            if let Ok(n) = str::parse::<usize>(name) {
                let expr = match pos.get(n) {
                    Some(expr) => *expr,
                    None => {
                        return Err(compile::Error::msg(
                            span,
                            format!("missing positional argument #{}", n),
                        ));
                    }
                };

                unused_pos.remove(&n);
                break 'expr ExprOrIdent::Expr(expr);
            }

            if let Some(n) = named.get(name.as_str()) {
                unused_named.remove(name.as_str());
                break 'expr ExprOrIdent::Expr(&n.expr);
            }

            let mut ident = cx.ident(name.as_str())?;
            ident.span = span;
            ExprOrIdent::Ident(ident)
        };

        let width = if !width.is_empty() {
            str::parse::<usize>(width).ok()
        } else {
            None
        };

        Ok(C::Format {
            expr,
            fill,
            align,
            width,
            precision,
            format_type,
            flags,
        })
    }

    fn parse_align(c: char) -> format::Alignment {
        match c {
            '<' => format::Alignment::Left,
            '^' => format::Alignment::Center,
            _ => format::Alignment::Right,
        }
    }
}

struct Iter<'a> {
    iter: str::CharIndices<'a>,
    a: Option<(usize, char)>,
    b: Option<(usize, char)>,
}

impl<'a> Iter<'a> {
    fn new(input: &'a str) -> Self {
        let mut iter = input.char_indices();
        let a = iter.next();
        let b = iter.next();
        Self { iter, a, b }
    }

    fn current(&self) -> Option<(usize, char, char)> {
        let (pos, a) = self.a?;
        let (_, b) = self.b.unwrap_or_default();
        Some((pos, a, b))
    }
}

impl Iterator for Iter<'_> {
    type Item = (usize, char, char);

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.current()?;

        self.a = self.b;
        self.b = self.iter.next();

        Some(value)
    }
}
