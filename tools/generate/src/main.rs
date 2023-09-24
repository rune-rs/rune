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
struct Syntax {
    variant: String,
    doc: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind")]
enum Token {
    #[serde(rename = "keyword")]
    Keyword(Keyword),
    #[serde(rename = "punct")]
    Punct(Punct),
    #[serde(rename = "syntax")]
    Syntax(Syntax),
}

impl Token {
    fn doc(&self) -> &str {
        match self {
            Self::Keyword(k) => &k.doc,
            Self::Punct(p) => &p.doc,
            Self::Syntax(p) => &p.doc,
        }
    }

    fn variant(&self) -> &str {
        match self {
            Self::Keyword(k) => &k.variant,
            Self::Punct(p) => &p.variant,
            Self::Syntax(p) => &p.variant,
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

    let syntax = tokens
        .iter()
        .flat_map(|t| match t {
            Token::Syntax(s) => Some(s),
            _ => None,
        })
        .collect::<Vec<_>>();

    // Collection of non-syntax tokens.
    let non_syntax = tokens
        .iter()
        .filter(|t| !matches!(t, Token::Syntax(..)))
        .collect::<Vec<_>>();

    let kind = &rust::import("crate::quote", "Kind");

    write_tokens(
        Path::new("crates/rune-macros/src/quote/generated.rs"),
        genco::quote!(
            $(format!("/// This file has been generated from `{}`", asset.display()))
            $("/// DO NOT modify by hand!")

            pub(crate) fn kind_from_ident(ident: &str) -> Option<$kind> {
                match ident {
                    $(for k in &keywords => $(quoted(&k.keyword)) => Some($kind($(quoted(&k.variant)))),$['\r'])
                    _ => None,
                }
            }

            pub(crate) fn kind_from_punct(buf: &[char]) -> Option<$kind> {
                match buf {
                    $(for p in &punctuations => $(buf_match(&p.punct)) => Some($kind($(quoted(&p.variant)))),$['\r'])
                    _ => None,
                }
            }
        ),
    )?;

    let copy_source = &rust::import("crate::ast", "CopySource");
    let delimiter = &rust::import("crate::ast", "Delimiter");
    let into_expectation = &rust::import("crate::parse", "IntoExpectation");
    let expectation = &rust::import("crate::parse", "Expectation");
    let display = &rust::import("core::fmt", "Display");
    let fmt_result = &rust::import("core::fmt", "Result");
    let formatter = &rust::import("core::fmt", "Formatter");
    let kind = &rust::import("crate::ast", "Kind");
    let lit_str_source = &rust::import("crate::ast", "StrSource");
    let macro_context = &rust::import("crate::macros", "MacroContext");
    let number_source = &rust::import("crate::ast", "NumberSource");
    let parse = &rust::import("crate::parse", "Parse");
    let compile_error = &rust::import("crate::compile", "Error");
    let compile_result = &rust::import("crate::compile", "Result");
    let parser = &rust::import("crate::parse", "Parser");
    let peeker = &rust::import("crate::parse", "Peeker");
    let peek = &rust::import("crate::parse", "Peek");
    let span = &rust::import("crate::ast", "Span");
    let spanned = &rust::import("crate::ast", "Spanned");
    let option_spanned = &rust::import("crate::ast", "OptionSpanned");
    let lit_source = &rust::import("crate::ast", "LitSource");
    let to_tokens = &rust::import("crate::macros", "ToTokens");
    let token = &rust::import("crate::ast", "Token");
    let token_stream = &rust::import("crate::macros", "TokenStream");
    let try_clone = &rust::import("crate::alloc::clone", "TryClone");
    let alloc = &rust::import("crate", "alloc");

    write_tokens(
        Path::new("crates/rune/src/ast/generated.rs"),
        genco::quote! {
            use crate as rune;
            $(format!("/// This file has been generated from `{}`", asset.display()))
            $("/// DO NOT modify by hand!")

            $(for t in &non_syntax join($['\n']) =>
                $(format!("/// {}", t.doc()))
                #[derive(Debug, $try_clone, Clone, Copy, PartialEq, Eq, Hash)]
                #[try_clone(copy)]
                #[non_exhaustive]
                pub struct $(t.variant()) {
                    $("/// Associated span.")
                    pub span: $span,
                }

                impl $spanned for $(t.variant()) {
                    fn span(&self) -> $span {
                        self.span
                    }
                }

                impl $option_spanned for $(t.variant()) {
                    fn option_span(&self) -> Option<$span> {
                        Some(self.span)
                    }
                }

                impl $parse for $(t.variant()) {
                    fn parse(p: &mut $parser<'_>) -> $compile_result<Self> {
                        let token = p.next()?;

                        match token.kind {
                            $kind::$(t.variant()) => Ok(Self { span: token.span }),
                            _ => Err($compile_error::expected(token, $kind::$(t.variant()))),
                        }
                    }
                }

                impl $peek for $(t.variant()) {
                    fn peek(peeker: &mut $peeker<'_>) -> bool {
                        matches!(peeker.nth(0), $kind::$(t.variant()))
                    }
                }

                impl $to_tokens for $(t.variant()) {
                    fn to_tokens(&self, _: &mut $macro_context<'_, '_, '_>, stream: &mut $token_stream) -> $alloc::Result<()> {
                        stream.push($token {
                            span: self.span,
                            kind: $kind::$(t.variant()),
                        })
                    }
                }
            )

            $("/// Helper macro to reference a specific token.")
            #[macro_export]
            macro_rules! T {
                ('(') => {
                    $$crate::ast::OpenParen
                };
                (')') => {
                    $$crate::ast::CloseParen
                };
                ('[') => {
                    $$crate::ast::OpenBracket
                };
                (']') => {
                    $$crate::ast::CloseBracket
                };
                ('{') => {
                    $$crate::ast::OpenBrace
                };
                ('}') => {
                    $$crate::ast::CloseBrace
                };
                (is not) => {
                    $$crate::ast::IsNot
                };
                $(for k in &keywords join($['\r']) =>
                    ($(&k.keyword)) => {
                        $$crate::ast::$(&k.variant)
                    };
                )
                $(for k in &punctuations join($['\r']) =>
                    ($(&k.punct)) => {
                        $$crate::ast::$(&k.variant)
                    };
                )
            }

            $("/// Helper macro to reference a specific token kind, or short sequence of kinds.")
            #[macro_export]
            macro_rules! K {
                (#!($$($$tt:tt)*)) => { $$crate::ast::Kind::Shebang($$($$tt)*) };
                (ident) => { $$crate::ast::Kind::Ident(..) };
                (ident ($$($$tt:tt)*)) => { $$crate::ast::Kind::Ident($$($$tt)*) };
                ('label) => { $$crate::ast::Kind::Label(..) };
                ('label ($$($$tt:tt)*)) => { $$crate::ast::Kind::Label($$($$tt)*) };
                (str) => { $$crate::ast::Kind::Str(..) };
                (str ($$($$tt:tt)*)) => { $$crate::ast::Kind::Str($$($$tt)*) };
                (bytestr) => { $$crate::ast::Kind::ByteStr(..) };
                (bytestr ($$($$tt:tt)*)) => { $$crate::ast::Kind::ByteStr($$($$tt)*) };
                (char) => { $$crate::ast::Kind::Char(..) };
                (char ($$($$tt:tt)*)) => { $$crate::ast::Kind::Char($$($$tt)*) };
                (byte) => { $$crate::ast::Kind::Byte(..) };
                (byte ($$($$tt:tt)*)) => { $$crate::ast::Kind::Byte($$($$tt)*) };
                (number) => { $$crate::ast::Kind::Number(..) };
                (number ($$($$tt:tt)*)) => { $$crate::ast::Kind::Number($$($$tt)*) };
                ('(') => { $$crate::ast::Kind::Open($$crate::ast::Delimiter::Parenthesis) };
                (')') => { $$crate::ast::Kind::Close($$crate::ast::Delimiter::Parenthesis) };
                ('[') => { $$crate::ast::Kind::Open($$crate::ast::Delimiter::Bracket) };
                (']') => { $$crate::ast::Kind::Close($$crate::ast::Delimiter::Bracket) };
                ('{') => { $$crate::ast::Kind::Open($$crate::ast::Delimiter::Brace) };
                ('}') => { $$crate::ast::Kind::Close($$crate::ast::Delimiter::Brace) };
                $(for k in &keywords join($['\r']) =>
                    ($(&k.keyword)) => { $$crate::ast::Kind::$(&k.variant) };
                )
                $(for k in &punctuations join($['\r']) =>
                    ($(&k.punct)) => { $$crate::ast::Kind::$(&k.variant) };
                )
            }

            $("/// The kind of the token.")
            #[derive(Debug, $try_clone, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub enum Kind {
                $("/// En end-of-file marker.")
                Eof,
                $("/// A single-line comment.")
                Comment,
                $("/// A multiline comment where the boolean indicates if it's been terminated correctly.")
                MultilineComment(bool),
                $("/// En error marker.")
                Error,
                $("/// The special initial line of a file shebang.")
                Shebang($lit_source),
                $("/// A close delimiter: `)`, `}`, or `]`.")
                Close($delimiter),
                $("/// An open delimiter: `(`, `{`, or `[`.")
                Open($delimiter),
                $("/// An identifier.")
                Ident($lit_source),
                $("/// A label, like `'loop`.")
                Label($lit_source),
                $("/// A byte literal.")
                Byte($copy_source<u8>),
                $("/// A byte string literal, including escape sequences. Like `b\"hello\\nworld\"`.")
                ByteStr($lit_str_source),
                $("/// A characer literal.")
                Char($copy_source<char>),
                $("/// A number literal, like `42` or `3.14` or `0xff`.")
                Number($number_source),
                $("/// A string literal, including escape sequences. Like `\"hello\\nworld\"`.")
                Str($lit_str_source),
                $(for t in &tokens join($['\r']) =>
                    $(format!("/// {}", t.doc()))
                    $(t.variant()),
                )
            }

            impl From<$token> for Kind {
                fn from(token: $token) -> Self {
                    token.kind
                }
            }

            impl Kind {
                $("/// Try to convert an identifier into a keyword.")
                pub(crate) fn from_keyword(ident: &str) -> Option<Self> {
                    match ident {
                        $(for k in &keywords join ($['\r']) => $(quoted(&k.keyword)) => Some(Self::$(&k.variant)),)
                        _ => None,
                    }
                }

                $("/// If applicable, convert this into a literal.")
                pub(crate) fn as_literal_str(&self) -> Option<&'static str> {
                    match self {
                        Self::Close(d) => Some(d.close()),
                        Self::Open(d) => Some(d.open()),
                        $(for k in &keywords join ($['\r']) => Self::$(&k.variant) => Some($(quoted(&k.keyword))),)
                        $(for p in &punctuations join ($['\r']) => Self::$(&p.variant) => Some($(quoted(&p.punct))),)
                        _ => None,
                    }
                }
            }

            impl $display for Kind {
                fn fmt(&self, f: &mut $formatter<'_>) -> $fmt_result {
                    $into_expectation::into_expectation(*self).fmt(f)
                }
            }

            impl $to_tokens for Kind {
                fn to_tokens(&self, context: &mut $macro_context<'_, '_, '_>, stream: &mut $token_stream) -> $alloc::Result<()> {
                    stream.push($token {
                        kind: *self,
                        span: context.macro_span(),
                    })
                }
            }

            impl $into_expectation for Kind {
                fn into_expectation(self) -> $expectation {
                    match self {
                        Self::Eof => $expectation::Description("eof"),
                        Self::Comment | Self::MultilineComment(..) => $expectation::Comment,
                        Self::Error => $expectation::Description("error"),
                        Self::Shebang { .. } => $expectation::Description("shebang"),
                        Self::Ident(..) => $expectation::Description("ident"),
                        Self::Label(..) => $expectation::Description("label"),
                        Self::Byte { .. } => $expectation::Description("byte literal"),
                        Self::ByteStr { .. } => $expectation::Description("byte string"),
                        Self::Char { .. } => $expectation::Description("char"),
                        Self::Number { .. } => $expectation::Description("number"),
                        Self::Str { .. } => $expectation::Description("string"),
                        Self::Close(delimiter) => $expectation::Delimiter(delimiter.close()),
                        Self::Open(delimiter) => $expectation::Delimiter(delimiter.open()),
                        $(for k in &keywords join ($['\r']) => Self::$(&k.variant) => $expectation::Keyword($(quoted(&k.keyword))),)
                        $(for p in &punctuations join ($['\r']) => Self::$(&p.variant) => $expectation::Punctuation($(quoted(&p.punct))),)
                        $(for s in &syntax join ($['\r']) => Self::$(&s.variant) => $expectation::Syntax,)
                    }
                }
            }
        },
    )?;

    Ok(())
}

fn buf_match(punct: &str) -> impl FormatInto<Rust> + '_ {
    genco::tokens::from_fn(move |mut tokens| {
        let chars = punct.chars().collect::<Vec<_>>();
        let len = chars.len();
        let extra = 3usize
            .checked_sub(len)
            .expect("a punctuation should not be longer than 3");
        let it = chars.into_iter().chain(std::iter::repeat('\0').take(extra));

        quote_in!(tokens => [$(for c in it join (, ) => $(format!("{:?}", c)))])
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
