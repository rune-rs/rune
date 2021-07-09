use proc_macro2 as p;
use proc_macro2::{Span, TokenStream, TokenTree};
use syn::Error;

mod builder;
mod generated;
mod inner;

use self::builder::Builder;
use self::inner::*;

pub struct Quote {
    ctx: &'static str,
    stream: &'static str,
}

impl Quote {
    /// Construct a new quote parser.
    pub fn new() -> Self {
        Self {
            ctx: "__rune_macros_ctx",
            stream: "__rune_macros_stream",
        }
    }

    /// Parse the given input stream and convert into code that constructs a
    /// `ToTokens` implementation.
    pub fn parse(&self, input: TokenStream) -> Result<TokenStream, Error> {
        let arg = (
            ("move", '|', self.ctx, ',', self.stream, '|'),
            braced(self.process(input)?),
        );

        let mut output = Builder::new();
        output.push((MACROS, S, "quote_fn", p(arg)));
        Ok(output.into_stream())
    }

    fn process(&self, input: TokenStream) -> Result<Builder, Error> {
        let mut output = Builder::new();

        let mut stack = vec![(p::Delimiter::None, input.into_iter().peekable())];

        while let Some((_, it)) = stack.last_mut() {
            let tt = match it.next() {
                Some(tt) => tt,
                None => {
                    let (d, _) = stack
                        .pop()
                        .ok_or_else(|| Error::new(Span::call_site(), "stack is empty"))?;

                    // Add the closing delimiter.
                    if let Some(variant) = Delimiter::from_proc_macro(d) {
                        self.encode_to_tokens(
                            Span::call_site(),
                            &mut output,
                            (Kind("Close"), p(variant)),
                        );
                    }

                    continue;
                }
            };

            match tt {
                TokenTree::Group(group) => {
                    // Add the opening delimiter.
                    if let Some(variant) = Delimiter::from_proc_macro(group.delimiter()) {
                        self.encode_to_tokens(
                            group.span(),
                            &mut output,
                            (Kind("Open"), p(variant)),
                        );
                    }

                    stack.push((group.delimiter(), group.stream().into_iter().peekable()));
                }
                TokenTree::Ident(ident) => {
                    // TODO: change Rune underscore from being a punctuation to
                    // an identifier to be in line with Rust.
                    if ident == "_" {
                        self.encode_to_tokens(ident.span(), &mut output, Kind("Underscore"));
                        continue;
                    }

                    let string = ident.to_string();

                    let kind = match generated::kind_from_ident(string.as_str()) {
                        Some(kind) => kind,
                        None => {
                            self.encode_to_tokens(ident.span(), &mut output, NewIdent(&string));
                            continue;
                        }
                    };

                    self.encode_to_tokens(ident.span(), &mut output, kind);
                }
                TokenTree::Punct(punct) => {
                    if punct.as_char() == '#'
                        && self.try_parse_expansion(&punct, &mut output, it)?
                    {
                        continue;
                    }

                    let mut buf = ['\0'; 3];
                    consume_punct(&punct, it, buf.iter_mut());

                    let kind = match generated::kind_from_punct(&buf) {
                        Some(kind) => kind,
                        _ => {
                            return Err(syn::Error::new(punct.span(), "unsupported punctuation"));
                        }
                    };

                    self.encode_to_tokens(punct.span(), &mut output, kind);
                }
                TokenTree::Literal(lit) => {
                    self.encode_to_tokens(lit.span(), &mut output, NewLit(lit));
                }
            }
        }

        Ok(output)
    }

    /// Try to parse an expansion.
    fn try_parse_expansion(
        &self,
        punct: &p::Punct,
        output: &mut Builder,
        it: &mut Peekable<impl Iterator<Item = TokenTree> + Clone>,
    ) -> Result<bool, Error> {
        // Clone for lookahead.
        let mut lh = it.clone();

        let next = match lh.next() {
            Some(next) => next,
            None => return Ok(false),
        };

        match next {
            // `#value` expansion.
            TokenTree::Ident(ident) => {
                self.encode_to_tokens(punct.span(), output, ident);
            }
            // `#(<expr>)<sep>*` repetition.
            TokenTree::Group(group) if group.delimiter() == p::Delimiter::Parenthesis => {
                let group = group.stream();

                // Parse the repitition character.
                let sep = match (lh.next(), lh.next()) {
                    (Some(sep), Some(TokenTree::Punct(p))) if p.as_char() == '*' => sep,
                    _ => return Ok(false),
                };

                output.push((
                    ("let", "mut", "it"),
                    '=',
                    ("IntoIterator", S, "into_iter", p(('&', group))),
                    ('.', "peekable", p(())),
                    ';',
                ));

                let body = (
                    (
                        ToTokensFn,
                        p(('&', "value", ',', self.ctx, ',', self.stream)),
                        ';',
                    ),
                    ("if", "it", '.', "peek", p(()), '.', "is_some", p(())),
                    braced(self.process(TokenStream::from(sep))?),
                );

                output.push((
                    ("while", "let", "Some", p("value")),
                    '=',
                    ("it", '.', "next", p(()), braced(body)),
                ));

                it.next();
                it.next();
                it.next();
                return Ok(true);
            }
            // Non-expansions.
            _ => return Ok(false),
        }

        it.next();
        Ok(true)
    }

    fn encode_to_tokens(&self, span: Span, output: &mut Builder, tokens: impl ToTokens) {
        output.push_spanned(
            span,
            (
                ToTokensFn,
                p(('&', tokens, ',', self.ctx, ',', self.stream)),
                ';',
            ),
        );
    }
}

use std::iter::Peekable;

fn consume_punct<'o>(
    initial: &p::Punct,
    it: &mut Peekable<impl Iterator<Item = TokenTree>>,
    mut out: impl Iterator<Item = &'o mut char>,
) {
    *out.next().unwrap() = initial.as_char();

    if !matches!(initial.spacing(), p::Spacing::Joint) {
        return;
    }

    for o in out {
        let (spacing, ch) = match it.peek() {
            Some(TokenTree::Punct(p)) => (p.spacing(), p.as_char()),
            _ => break,
        };

        *o = ch;

        it.next();
        if !matches!(spacing, p::Spacing::Joint) {
            break;
        }
    }
}
