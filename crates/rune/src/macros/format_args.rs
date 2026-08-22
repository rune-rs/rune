use core::str;

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::{self, BTreeMap, BTreeSet, Box, HashMap, String, Vec};
use crate::ast::{self, Span};
use crate::compile::{self, WithSpan};
use crate::macros::{quote, MacroContext, Quote, ToTokens, TokenStream};
use crate::runtime::format;

/// A format specification: A format string followed by arguments to be
/// formatted in accordance with that string.
///
/// This type can only be built inside of a macro context since it performs
/// constant evaluation.
///
/// Both the format string and the arguments are held as the tokens they were
/// written as rather than as a syntax tree, so nothing here recurses over what
/// a macro was handed - see [`MacroContext::exprs`].
pub struct FormatArgs {
    /// The format string.
    format: TokenStream,
    /// The span of the format string.
    format_span: Span,
    /// Format arguments.
    args: Vec<FormatArg>,
}

impl FormatArgs {
    /// Parse format arguments out of the whole input of a macro.
    ///
    /// # Examples
    ///
    /// ```
    /// # use rune::support::*;
    /// use rune::macros::{self, quote, FormatArgs};
    ///
    /// macros::test(|cx| {
    ///     let stream = quote!("Hello {}", 42).into_token_stream(cx)?;
    ///     let args = FormatArgs::parse(cx, &stream)?;
    ///     let expanded = args.expand(cx)?.into_token_stream(cx)?;
    ///     assert!(expanded.kinds().count() > 0);
    ///     Ok(())
    /// })?;
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn parse(cx: &mut MacroContext<'_, '_, '_>, stream: &TokenStream) -> compile::Result<Self> {
        let exprs = cx.exprs(stream)?;
        Self::from_exprs(cx, exprs)
    }

    /// Build format arguments out of expressions which have already been split
    /// out of the input of a macro with [`MacroContext::exprs`].
    ///
    /// This is what a macro whose format specification is preceded by
    /// arguments of its own uses, `assert!` being one.
    pub fn from_exprs<I>(cx: &mut MacroContext<'_, '_, '_>, exprs: I) -> compile::Result<Self>
    where
        I: IntoIterator<Item = TokenStream>,
    {
        let mut it = exprs.into_iter();

        let Some(format) = it.next() else {
            return Err(compile::Error::msg(
                cx.input_span(),
                "expected format specifier",
            ));
        };

        let format_span = cx.stream_span(&format);

        let mut args = Vec::new();

        for expr in it {
            args.try_push(FormatArg::new(cx, expr)?)?;
        }

        Ok(Self {
            format,
            format_span,
            args,
        })
    }

    /// Expand the format specification.
    pub fn expand(&self, cx: &mut MacroContext<'_, '_, '_>) -> compile::Result<Quote<'_>> {
        let format = cx.eval_stream(&self.format)?;

        let mut pos = Vec::new();
        let mut named = HashMap::<Box<str>, _>::new();

        for a in &self.args {
            match &a.name {
                None => {
                    if !named.is_empty() {
                        return Err(compile::Error::msg(
                            a.span,
                            "unnamed positional arguments must come before named ones",
                        ));
                    }

                    pos.try_push(a)?;
                }
                Some(name) => {
                    named.try_insert(name.try_clone()?, a)?;
                }
            }
        }

        let format = format.downcast::<String>().with_span(self.format_span)?;

        let mut unused_pos = (0..pos.len()).try_collect::<BTreeSet<_>>()?;
        let mut unused_named = named
            .iter()
            .map(|(key, n)| Ok::<_, alloc::Error>((key.try_clone()?, n.span)))
            .try_collect::<alloc::Result<BTreeMap<_, _>>>()??;

        let result = expand_format_spec(
            cx,
            self.format_span,
            &format,
            &pos,
            &mut unused_pos,
            &named,
            &mut unused_named,
        );

        let expanded = match result {
            Ok(expanded) => expanded,
            Err(message) => return Err(compile::Error::msg(self.format_span, message)),
        };

        if let Some(span) = unused_pos
            .into_iter()
            .flat_map(|n| pos.get(n))
            .map(|a| a.span)
            .next()
        {
            return Err(compile::Error::msg(span, "unused positional argument"));
        }

        if let Some((key, span)) = unused_named.into_iter().next() {
            return Err(compile::Error::msg(
                span,
                format!("unused named argument `{key}`"),
            ));
        }

        Ok(expanded)
    }
}

/// A single format argument.
struct FormatArg {
    /// The name of the argument, if it was written as `name = value`.
    name: Option<Box<str>>,
    /// The tokens the value of the argument was written as.
    value: TokenStream,
    /// The span of the argument as a whole.
    span: Span,
}

impl FormatArg {
    /// Classify one of the expressions a macro's input was split into.
    ///
    /// An argument is named if it starts with `ident =`, which is decided by
    /// looking at the two tokens it starts with rather than by parsing it.
    fn new(cx: &mut MacroContext<'_, '_, '_>, expr: TokenStream) -> compile::Result<Self> {
        let span = cx.stream_span(&expr);

        let mut it = (&expr).into_iter();

        let key = match (it.next(), it.next()) {
            (Some(key), Some(eq)) if matches!(eq.kind, ast::Kind::Eq) => match key.kind {
                ast::Kind::Ident(source) => Some(ast::Ident {
                    span: key.span,
                    source,
                }),
                _ => None,
            },
            _ => None,
        };

        let Some(key) = key else {
            return Ok(Self {
                name: None,
                value: expr,
                span,
            });
        };

        let name = cx.resolve(key)?.try_into()?;

        let mut value = TokenStream::new();

        for token in expr.into_iter().skip(2) {
            value.push(token)?;
        }

        Ok(Self {
            name: Some(name),
            value,
            span,
        })
    }
}

fn expand_format_spec<'a>(
    cx: &mut MacroContext<'_, '_, '_>,
    span: Span,
    input: &str,
    pos: &[&'a FormatArg],
    unused_pos: &mut BTreeSet<usize>,
    named: &HashMap<Box<str>, &'a FormatArg>,
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
        Expr(&'a TokenStream),
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

    /// The largest width or precision a value can be written with.
    ///
    /// A precision is handed to the formatter the platform provides, which
    /// takes one that fits in a `u16` and panics on anything larger. A width is
    /// written out here instead, but it is held to the same bound so that a
    /// specification is not accepted under one and rejected under the other.
    const MAX_FORMAT_ARGUMENT: usize = u16::MAX as usize;

    /// Bound a width or a precision which was given as a number.
    fn bound_format_argument(span: Span, what: &str, n: usize) -> compile::Result<usize> {
        if n > MAX_FORMAT_ARGUMENT {
            return Err(compile::Error::msg(
                span,
                format!("{what} {n} is larger than the maximum of {MAX_FORMAT_ARGUMENT}"),
            ));
        }

        Ok(n)
    }

    /// Parse the digits a width or a precision was written as.
    ///
    /// A number too large to use was dropped rather than reported, so a
    /// specification carrying one was honoured as if it had not been written.
    fn parse_format_argument(span: Span, what: &str, digits: &str) -> compile::Result<usize> {
        // Digits which do not parse are ones too large to hold, which is past
        // the bound either way.
        let Ok(n) = str::parse::<usize>(digits) else {
            return Err(compile::Error::msg(
                span,
                format!("{what} {digits} is larger than the maximum of {MAX_FORMAT_ARGUMENT}"),
            ));
        };

        bound_format_argument(span, what, n)
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
        pos: &[&'a FormatArg],
        unused_pos: &mut BTreeSet<usize>,
        named: &HashMap<Box<str>, &'a FormatArg>,
        unused_named: &mut BTreeMap<Box<str>, Span>,
    ) -> compile::Result<C<'a>> {
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
                                format!("unsupported char `{c}` in spec"),
                            ));
                        }
                    }

                    iter.next();
                    break;
                }
            }
        }

        let precision = if input_precision {
            let &arg = match pos.get(*count) {
                Some(arg) => arg,
                None => {
                    return Err(compile::Error::msg(
                        span,
                        format!(
                            "missing positional argument #{count} \
                            which is required for position parameter",
                        ),
                    ));
                }
            };

            unused_pos.remove(count);

            let value = cx.eval_stream(&arg.value)?;
            let precision = value.as_usize().with_span(span)?;

            *count += 1;
            Some(bound_format_argument(span, "precision", precision)?)
        } else if !precision.is_empty() {
            Some(parse_format_argument(span, "precision", precision)?)
        } else {
            None
        };

        let expr = 'expr: {
            if name.is_empty() {
                let Some(arg) = pos.get(*count) else {
                    return Err(compile::Error::msg(
                        span,
                        format!("missing positional argument #{count}"),
                    ));
                };

                unused_pos.remove(count);
                *count += 1;
                break 'expr ExprOrIdent::Expr(&arg.value);
            };

            if let Ok(n) = str::parse::<usize>(name) {
                let arg = match pos.get(n) {
                    Some(arg) => *arg,
                    None => {
                        return Err(compile::Error::msg(
                            span,
                            format!("missing positional argument #{n}"),
                        ));
                    }
                };

                unused_pos.remove(&n);
                break 'expr ExprOrIdent::Expr(&arg.value);
            }

            if let Some(n) = named.get(name.as_str()) {
                unused_named.remove(name.as_str());
                break 'expr ExprOrIdent::Expr(&n.value);
            }

            let mut ident = cx.ident(name.as_str())?;
            ident.span = span;
            ExprOrIdent::Ident(ident)
        };

        let width = if !width.is_empty() {
            Some(parse_format_argument(span, "width", width)?)
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
