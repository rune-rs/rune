use proc_macro2 as p;
use proc_macro2::{Span, TokenStream, TokenTree};
use syn::Error;

mod builder;
mod generated;
mod inner;

pub(crate) trait ToTokens {
    fn to_tokens(self, stream: &mut TokenStream, span: Span);
}

impl ToTokens for p::Ident {
    fn to_tokens(self, stream: &mut TokenStream, _: Span) {
        stream.extend(std::iter::once(TokenTree::Ident(self)));
    }
}

use self::builder::Builder;
use self::inner::{
    Delimiter, Group, Ident, Kind, KindVariant, NewIdent, NewLit, Punct, RUNE_MACROS,
};

pub struct Quote {
    ctx: Ident<'static>,
    stream: Ident<'static>,
}

impl Quote {
    /// Construct a new quote parser.
    pub fn new() -> Self {
        Self {
            ctx: Ident::new("__rune_macros_ctx"),
            stream: Ident::new("__rune_macros_stream"),
        }
    }

    /// Parse the given input stream and convert into code that constructs a
    /// `ToTokens` implementation.
    pub fn parse(&self, input: TokenStream) -> Result<TokenStream, Error> {
        let mut output = Builder::new();

        let mut inner = Builder::new();
        inner.push(Ident::new("move"));
        inner.push(Punct::new("|"));
        inner.push(self.ctx);
        inner.push(Punct::new(","));
        inner.push(self.stream);
        inner.push(Punct::new("|"));
        inner.push(Group::new(p::Delimiter::Brace, self.process(input)?));

        output.push(RUNE_MACROS);
        output.push(Ident::new("quote_fn"));

        output.push(Group::new(p::Delimiter::Parenthesis, inner));

        Ok(output.into_stream())
    }

    fn process(&self, input: TokenStream) -> Result<Builder, Error> {
        let mut output = Builder::new();

        let mut stack = Vec::new();
        stack.push((p::Delimiter::None, input.into_iter().peekable()));

        while let Some((_, it)) = stack.last_mut() {
            let tt = match it.next() {
                Some(tt) => tt,
                None => {
                    let (d, _) = stack
                        .pop()
                        .ok_or_else(|| Error::new(Span::call_site(), "stack is empty"))?;

                    if let Some(variant) = Delimiter::from_proc_macro(d) {
                        self.encode_to_tokens(
                            Span::call_site(),
                            &mut output,
                            KindVariant {
                                kind: Kind::new("Close"),
                                variant,
                            },
                        );
                    }

                    continue;
                }
            };

            match tt {
                TokenTree::Group(group) => {
                    if let Some(variant) = Delimiter::from_proc_macro(group.delimiter()) {
                        self.encode_to_tokens(
                            group.span(),
                            &mut output,
                            KindVariant {
                                kind: Kind::new("Open"),
                                variant,
                            },
                        );
                    }

                    stack.push((group.delimiter(), group.stream().into_iter().peekable()));
                }
                TokenTree::Ident(ident) => {
                    if ident == "_" {
                        self.encode_to_tokens(ident.span(), &mut output, Kind::new("Underscore"));
                        continue;
                    }

                    let string = ident.to_string();

                    let kind = match generated::kind_from_ident(string.as_str()) {
                        Some(kind) => kind,
                        None => {
                            self.encode_to_tokens(
                                ident.span(),
                                &mut output,
                                NewIdent::new(&string),
                            );
                            continue;
                        }
                    };

                    self.encode_to_tokens(ident.span(), &mut output, kind);
                }
                TokenTree::Punct(punct) => {
                    if punct.as_char() == '#' && self.try_parse_expansion(&punct, &mut output, it) {
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
                    self.encode_to_tokens(lit.span(), &mut output, NewLit::new(lit));
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
        it: &mut Peekable<impl Iterator<Item = TokenTree>>,
    ) -> bool {
        let next = match it.peek() {
            Some(next) => next,
            None => return false,
        };

        match next {
            TokenTree::Ident(ident) => {
                self.encode_to_tokens(punct.span(), output, ident.clone());
            }
            // Token group parsing, currently disabled because it's not
            // particularly useful.
            /*TokenTree::Group(group) if group.delimiter() == p::Delimiter::Parenthesis => {
                let group = Builder::from(group.stream());
                let group = Group::new(p::Delimiter::Parenthesis, group);
                self.encode_to_tokens(punct.span(), output, group);
            }*/
            _ => return false,
        }

        it.next();
        true
    }

    fn encode_to_tokens(&self, span: Span, output: &mut Builder, tokens: impl ToTokens) {
        let mut args = Builder::spanned(span);

        args.push(Punct::joint("&"));
        args.push(tokens);
        args.push(Punct::new(","));
        args.push(self.ctx);
        args.push(Punct::new(","));
        args.push(self.stream);

        output.push(inner::ToTokensFn);
        output.push(Group::new(p::Delimiter::Parenthesis, args));
        output.push(Punct::new(";"));
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
        let p = match it.peek() {
            Some(TokenTree::Punct(p)) => p,
            _ => break,
        };

        *o = p.as_char();

        if matches!(p.spacing(), p::Spacing::Joint) {
            it.next();
        } else {
            it.next();
            break;
        }
    }
}
