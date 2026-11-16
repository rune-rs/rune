//! Split source text into classified spans for syntax highlighting.
//!
//! This uses the lexer in the mode the formatter does, so it keeps whitespace
//! and comments and copes with source that does not compile.

use crate::alloc::{self, Vec};
use crate::ast::{Kind, Span};
use crate::parse::Lexer;
use crate::SourceId;

/// The class of a [`Token`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Class {
    /// A keyword, including `true`, `false` and `self`.
    Keyword,
    /// An identifier or label.
    Ident,
    /// A number literal.
    Number,
    /// A string or byte string literal.
    String,
    /// A character literal.
    Char,
    /// A byte literal.
    Byte,
    /// A template literal.
    Template,
    /// A comment which is not a doc comment.
    Comment,
    /// A doc comment.
    DocComment,
    /// An attribute, from `#` to its closing bracket.
    Attribute,
    /// An identifier immediately followed by `!`.
    Macro,
    /// Delimiters and separators.
    Punctuation,
    /// Operators.
    Operator,
    /// Whitespace.
    Whitespace,
    /// Text which could not be lexed.
    Error,
}

impl Class {
    /// A stable lowercase name for the class.
    pub fn as_str(self) -> &'static str {
        match self {
            Class::Keyword => "keyword",
            Class::Ident => "ident",
            Class::Number => "number",
            Class::String => "string",
            Class::Char => "char",
            Class::Byte => "byte",
            Class::Template => "template",
            Class::Comment => "comment",
            Class::DocComment => "doc-comment",
            Class::Attribute => "attribute",
            Class::Macro => "macro",
            Class::Punctuation => "punctuation",
            Class::Operator => "operator",
            Class::Whitespace => "whitespace",
            Class::Error => "error",
        }
    }
}

/// A classified span of source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    /// The class of the span.
    pub class: Class,
    /// The byte span in the source.
    pub span: Span,
}

/// Classify all of `source`.
///
/// The tokens are in order and cover every byte exactly once. If the source
/// stops lexing partway through, the rest of it is one [`Class::Error`] token.
///
/// # Examples
///
/// ```
/// use rune::highlight::{highlight, Class};
///
/// let tokens = highlight("fn main() {}")?;
/// assert_eq!(tokens[0].class, Class::Keyword);
/// # Ok::<_, rune::alloc::Error>(())
/// ```
pub fn highlight(source: &str) -> alloc::Result<Vec<Token>> {
    let mut lexer = Lexer::new(source, SourceId::new(0), true).without_processing();
    let mut tokens = Vec::new();
    let mut tags = Vec::new();
    let mut pos = 0;

    loop {
        let token = match lexer.next() {
            Ok(Some(token)) => token,
            Ok(None) => break,
            Err(..) => {
                if pos < source.len() {
                    tokens.try_push(Token {
                        class: Class::Error,
                        span: Span::new(pos, source.len()),
                    })?;
                    tags.try_push(Kind::Error)?;
                }

                pos = source.len();
                break;
            }
        };

        let start = token.span.start.into_usize().max(pos);
        let end = token.span.end.into_usize().min(source.len());

        if start > pos {
            tokens.try_push(Token {
                class: Class::Error,
                span: Span::new(pos, start),
            })?;
            tags.try_push(Kind::Error)?;
        }

        if end <= start {
            continue;
        }

        let class = classify(token.kind, source.get(start..end).unwrap_or_default());
        tokens.try_push(Token {
            class,
            span: Span::new(start, end),
        })?;
        tags.try_push(token.kind)?;
        pos = end;
    }

    if pos < source.len() {
        tokens.try_push(Token {
            class: Class::Error,
            span: Span::new(pos, source.len()),
        })?;
        tags.try_push(Kind::Error)?;
    }

    refine(&mut tokens, &tags);
    Ok(tokens)
}

/// Mark macro invocations and attributes, which need to look at neighbours.
fn refine(tokens: &mut [Token], tags: &[Kind]) {
    let mut i = 0;

    while i < tokens.len() {
        match tags[i] {
            Kind::Ident(..) if matches!(tags.get(i + 1), Some(Kind::Bang)) => {
                tokens[i].class = Class::Macro;
            }
            Kind::Pound => {
                let mut open = i + 1;

                if matches!(tags.get(open), Some(Kind::Bang)) {
                    open += 1;
                }

                if matches!(
                    tags.get(open),
                    Some(Kind::Open(crate::ast::Delimiter::Bracket))
                ) {
                    let mut depth = 0usize;
                    let mut end = open;

                    while end < tags.len() {
                        match tags[end] {
                            Kind::Open(..) => depth += 1,
                            Kind::Close(..) => {
                                depth = depth.saturating_sub(1);

                                if depth == 0 {
                                    break;
                                }
                            }
                            _ => (),
                        }

                        end += 1;
                    }

                    let end = end.min(tags.len() - 1);

                    for token in &mut tokens[i..=end] {
                        if !matches!(token.class, Class::Whitespace | Class::Comment) {
                            token.class = Class::Attribute;
                        }
                    }

                    i = end;
                }
            }
            _ => (),
        }

        i += 1;
    }
}

fn classify(kind: Kind, text: &str) -> Class {
    match kind {
        Kind::Whitespace => Class::Whitespace,
        Kind::Comment | Kind::MultilineComment(..) => {
            if is_doc_comment(text) {
                Class::DocComment
            } else {
                Class::Comment
            }
        }
        Kind::Shebang(..) => Class::Comment,
        Kind::Error => Class::Error,
        Kind::Ident(..) | Kind::Label(..) => Class::Ident,
        Kind::Number(..) => Class::Number,
        Kind::Str(..) | Kind::ByteStr(..) => Class::String,
        Kind::Char(..) => Class::Char,
        Kind::Byte(..) => Class::Byte,
        Kind::TemplateString => Class::Template,
        Kind::Abstract
        | Kind::AlignOf
        | Kind::As
        | Kind::Async
        | Kind::Await
        | Kind::Become
        | Kind::Break
        | Kind::Const
        | Kind::Continue
        | Kind::Crate
        | Kind::Default
        | Kind::Do
        | Kind::Else
        | Kind::Enum
        | Kind::Extern
        | Kind::False
        | Kind::Final
        | Kind::Fn
        | Kind::For
        | Kind::If
        | Kind::Impl
        | Kind::In
        | Kind::Is
        | Kind::Let
        | Kind::Loop
        | Kind::Macro
        | Kind::Match
        | Kind::Mod
        | Kind::Move
        | Kind::Mut
        | Kind::Not
        | Kind::OffsetOf
        | Kind::Override
        | Kind::Priv
        | Kind::Proc
        | Kind::Pub
        | Kind::Pure
        | Kind::Ref
        | Kind::Return
        | Kind::Select
        | Kind::SelfType
        | Kind::SelfValue
        | Kind::SizeOf
        | Kind::Static
        | Kind::Struct
        | Kind::Super
        | Kind::True
        | Kind::TypeOf
        | Kind::Unsafe
        | Kind::Use
        | Kind::Virtual
        | Kind::While
        | Kind::Yield => Class::Keyword,
        Kind::Amp
        | Kind::AmpAmp
        | Kind::AmpEq
        | Kind::Bang
        | Kind::BangEq
        | Kind::Caret
        | Kind::CaretEq
        | Kind::Dash
        | Kind::DashEq
        | Kind::Div
        | Kind::DotDot
        | Kind::DotDotEq
        | Kind::Eq
        | Kind::EqEq
        | Kind::Gt
        | Kind::GtEq
        | Kind::GtGt
        | Kind::GtGtEq
        | Kind::Lt
        | Kind::LtEq
        | Kind::LtLt
        | Kind::LtLtEq
        | Kind::Perc
        | Kind::PercEq
        | Kind::Pipe
        | Kind::PipeEq
        | Kind::PipePipe
        | Kind::Plus
        | Kind::PlusEq
        | Kind::SlashEq
        | Kind::Star
        | Kind::StarEq => Class::Operator,
        _ => Class::Punctuation,
    }
}

fn is_doc_comment(text: &str) -> bool {
    if let Some(rest) = text.strip_prefix("//") {
        return rest.starts_with("!") || (rest.starts_with('/') && !rest.starts_with("//"));
    }

    if let Some(rest) = text.strip_prefix("/*") {
        return rest.starts_with('!')
            || (rest.starts_with('*') && !rest.starts_with("**") && !rest.starts_with("*/"));
    }

    false
}
