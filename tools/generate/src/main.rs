use anyhow::{Context as _, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

use genco::prelude::*;

#[derive(Debug, Deserialize)]
struct Keyword {
    variant: String,
    doc: String,
    keyword: String,
}

#[derive(Debug, Deserialize)]
struct Punct {
    variant: String,
    doc: String,
    punct: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind")]
enum Token {
    #[serde(rename = "keyword")]
    Keyword(Keyword),
    #[serde(rename = "punct")]
    Punct(Punct),
}

impl Token {
    fn doc(&self) -> &str {
        match self {
            Token::Keyword(k) => &k.doc,
            Token::Punct(p) => &p.doc,
        }
    }

    fn variant(&self) -> &str {
        match self {
            Token::Keyword(k) => &k.variant,
            Token::Punct(p) => &p.variant,
        }
    }

    fn desc(&self) -> &str {
        match self {
            Token::Keyword(k) => &k.keyword,
            Token::Punct(p) => &p.punct,
        }
    }
}

fn main() -> Result<()> {
    let asset = Path::new("assets").join("tokens.yaml");
    let f = fs::File::open(&asset).context("opening asset file")?;
    let tokens: Vec<Token> = serde_yaml::from_reader(f).context("reading yaml")?;

    let keywords = tokens
        .iter()
        .flat_map(|t| match t {
            Token::Keyword(k) => Some(k),
            _ => None,
        })
        .collect::<Vec<_>>();

    let punctuations = tokens
        .iter()
        .flat_map(|t| match t {
            Token::Punct(p) => Some(p),
            _ => None,
        })
        .collect::<Vec<_>>();

    let kind = &rust::import("crate::quote", "Kind");

    write_tokens(
        Path::new("crates/rune-macros/src/quote/generated.rs"),
        genco::quote!(
            #(format!("/// This file has been generated from `{}`", asset.display()))
            #("/// DO NOT modify by hand!")

            pub(crate) fn kind_from_ident(ident: &str) -> Option<#kind> {
                match ident {
                    #(for k in &keywords => #(quoted(&k.keyword)) => Some(#kind::new(#(quoted(&k.variant)))),#<push>)
                    _ => None,
                }
            }

            pub(crate) fn kind_from_punct(buf: &[char]) -> Option<#kind> {
                match buf {
                    #(for p in &punctuations => #(buf_match(&p.punct)) => Some(#kind::new(#(quoted(&p.variant)))),#<push>)
                    _ => None,
                }
            }
        ),
    )?;

    let copy_source = &rust::import("crate::ast", "CopySource");
    let delimiter = &rust::import("crate::ast", "Delimiter");
    let description = &rust::import("crate::shared", "Description");
    let display = &rust::import("std::fmt", "Display");
    let fmt_result = &rust::import("std::fmt", "Result");
    let formatter = &rust::import("std::fmt", "Formatter");
    let kind = &rust::import("crate::ast", "Kind");
    let lit_str_source = &rust::import("crate::ast", "LitStrSource");
    let macro_context= &rust::import("crate::macros", "MacroContext");
    let number_source = &rust::import("crate::ast", "NumberSource");
    let parse = &rust::import("crate::parsing", "Parse");
    let parse_error = &rust::import("crate::parsing", "ParseError");
    let parse_error_kind = &rust::import("crate::parsing", "ParseErrorKind");
    let parser = &rust::import("crate::parsing", "Parser");
    let peeker = &rust::import("crate::parsing", "Peeker");
    let peek = &rust::import("crate::parsing", "Peek");
    let span = &rust::import("runestick", "Span");
    let spanned = &rust::import("crate", "Spanned");
    let string_source = &rust::import("crate::ast", "StringSource");
    let to_tokens= &rust::import("crate::macros", "ToTokens");
    let token = &rust::import("crate::ast", "Token");
    let token_stream = &rust::import("crate::macros", "TokenStream");

    write_tokens(
        Path::new("crates/rune/src/ast/generated.rs"),
        genco::quote!{
            #(format!("/// This file has been generated from `{}`", asset.display()))
            #("/// DO NOT modify by hand!")

            #(for t in &tokens join(#<line>) =>
                #(format!("/// {}", t.doc()))
                #[derive(Debug, Clone, Copy, PartialEq, Eq)]
                pub struct #(t.variant()) {
                    #("/// Associated token.")
                    pub token: #token,
                }

                impl #spanned for #(t.variant()) {
                    fn span(&self) -> #span {
                        self.token.span()
                    }
                }

                impl #parse for #(t.variant()) {
                    fn parse(p: &mut #parser<'_>) -> Result<Self, #parse_error> {
                        let token = p.next()?;

                        match token.kind {
                            #kind::#(t.variant()) => Ok(Self {
                                token,
                            }),
                            _ => Err(#parse_error::new(token, #parse_error_kind::TokenMismatch {
                                expected: #kind::#(t.variant()),
                                actual: token.kind,
                            })),
                        }
                    }
                }

                impl #peek for #(t.variant()) {
                    fn peek(peeker: &mut #peeker<'_>) -> bool {
                        matches!(peeker.nth(0), #kind::#(t.variant()))
                    }
                }

                impl #to_tokens for #(t.variant()) {
                    fn to_tokens(&self, _: &#macro_context, stream: &mut #token_stream) {
                        stream.push(self.token);
                    }
                }
            )

            #("/// Helper macro to reference a specific token.")
            #[macro_export]
            macro_rules! T {
                (()) => { $crate::ast::LitUnit };
                ('(') => { $crate::ast::OpenParen };
                (')') => { $crate::ast::CloseParen };
                ('[') => { $crate::ast::OpenBracket };
                (']') => { $crate::ast::CloseBracket };
                ('{') => { $crate::ast::OpenBrace };
                ('}') => { $crate::ast::CloseBrace };
                #(for k in &keywords join(#<push>) =>
                    (#(&k.keyword)) => { $crate::ast::generated::#(&k.variant) };
                )
                #(for k in &punctuations join(#<push>) =>
                    (#(&k.punct)) => { $crate::ast::generated::#(&k.variant) };
                )
            }

            #("/// Helper macro to reference a specific token kind, or short sequence of kinds.")
            #[macro_export]
            macro_rules! K {
                (ident ($($tt:tt)*)) => { $crate::ast::Kind::Ident($($tt)*) };
                ('(') => { $crate::ast::Kind::Open($crate::ast::Delimiter::Parenthesis) };
                (')') => { $crate::ast::Kind::Close($crate::ast::Delimiter::Parenthesis) };
                ('[') => { $crate::ast::Kind::Open($crate::ast::Delimiter::Bracket) };
                (']') => { $crate::ast::Kind::Close($crate::ast::Delimiter::Bracket) };
                ('{') => { $crate::ast::Kind::Open($crate::ast::Delimiter::Brace) };
                ('}') => { $crate::ast::Kind::Close($crate::ast::Delimiter::Brace) };
                #(for k in &keywords join(#<push>) =>
                    (#(&k.keyword)) => { $crate::ast::Kind::#(&k.variant) };
                )
                #(for k in &punctuations join(#<push>) =>
                    (#(&k.punct)) => { $crate::ast::Kind::#(&k.variant) };
                )
            }

            #("/// The kind of the token.")
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub enum Kind {
                #("/// En end-of-file marker.")
                Eof,
                #("/// En error marker.")
                Error,
                #("/// A close delimiter: `)`, `}`, or `]`.")
                Close(#delimiter),
                #("/// An open delimiter: `(`, `{`, or `[`.")
                Open(#delimiter),
                #("/// An identifier.")
                Ident(#string_source),
                #("/// A label, like `'loop`.")
                Label(#string_source),
                #("/// A byte literal.")
                LitByte(#copy_source<u8>),
                #("/// A byte string literal, including escape sequences. Like `b\"hello\\nworld\"`.")
                LitByteStr(#lit_str_source),
                #("/// A characer literal.")
                LitChar(#copy_source<char>),
                #("/// A number literal, like `42` or `3.14` or `0xff`.")
                LitNumber(#number_source),
                #("/// A string literal, including escape sequences. Like `\"hello\\nworld\"`.")
                LitStr(#lit_str_source),
                #(for t in &tokens join(#<push>) =>
                    #(format!("/// {}", t.doc()))
                    #(t.variant()),
                )
            }

            impl From<#token> for Kind {
                fn from(token: #token) -> Self {
                    token.kind
                }
            }

            impl Kind {
                #("/// Try to convert an identifier into a keyword.")
                pub fn from_keyword(ident: &str) -> Option<Self> {
                    match ident {
                        #(for k in &keywords join (#<push>) => #(quoted(&k.keyword)) => Some(Self::#(&k.variant)),)
                        _ => None,
                    }
                }

                #("/// Get the kind as a descriptive string.")
                fn as_str(self) -> &'static str {
                    match self {
                        Self::Eof => "eof",
                        Self::Error => "error",
                        Self::Close(delimiter) => delimiter.close(),
                        Self::Open(delimiter) => delimiter.open(),
                        Self::Ident(..) => "ident",
                        Self::Label(..) => "label",
                        Self::LitByte { .. } => "byte",
                        Self::LitByteStr { .. } => "byte string",
                        Self::LitChar { .. } => "char",
                        Self::LitNumber { .. } => "number",
                        Self::LitStr { .. } => "string",
                        #(for t in &tokens join (#<push>) => Self::#(t.variant()) => #(quoted(t.desc())),)
                    }
                }
            }

            impl #display for Kind {
                fn fmt(&self, f: &mut #formatter<'_>) -> #fmt_result {
                    f.write_str(self.as_str())
                }
            }

            impl #to_tokens for Kind {
                fn to_tokens(&self, context: &#macro_context, stream: &mut #token_stream) {
                    stream.push(#token {
                        kind: *self,
                        span: context.span(),
                    });
                }
            }

            impl #description for Kind {
                fn description(self) -> &'static str {
                    self.as_str()
                }
            }
        },
    )?;

    Ok(())
}

fn buf_match<'a>(punct: &'a str) -> impl FormatInto<Rust> + 'a {
    genco::tokens::from_fn(move |mut tokens| {
        let chars = punct.chars().collect::<Vec<_>>();
        let len = chars.len();
        let extra = 3usize
            .checked_sub(len)
            .expect("a punctuation should not be longer than 3");
        let it = chars.into_iter().chain(std::iter::repeat('\0').take(extra));

        quote_in!(tokens => [#(for c in it join (, ) => #(format!("{:?}", c)))])
    })
}

fn write_tokens(output: &Path, tokens: rust::Tokens) -> Result<()> {
    use genco::fmt;

    println!("writing: {}", output.display());

    let fmt = fmt::Config::from_lang::<Rust>().with_indentation(fmt::Indentation::Space(4));

    let out = fs::File::create(output).context("opening output file")?;
    let mut w = fmt::IoWriter::new(out);

    let config = rust::Config::default().with_default_import(rust::ImportMode::Qualified);

    tokens.format_file(&mut w.as_formatter(&fmt), &config)?;
    Ok(())
}
