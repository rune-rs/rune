use core::ascii;
use core::fmt;
use core::ops::Neg;

use crate::ast::prelude::*;
use crate::ast::{Kind, Span, Spanned};
use crate::compile;
use crate::macros::{MacroContext, SyntheticId, ToTokens, TokenStream};
use crate::parse::{Expectation, IntoExpectation, Parse, Parser, Peek};
use crate::SourceId;

/// A single token encountered during parsing.
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct Token {
    /// The span of the token.
    pub span: Span,
    /// The kind of the token.
    pub kind: Kind,
}

impl Token {
    /// Format the current token to a formatter.
    pub(crate) fn token_fmt(&self, cx: &MacroContext, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            Kind::Eof | Kind::Error => {
                // NB: marker tokens can't be formatted.
                return Err(fmt::Error);
            }
            Kind::Ident(s) => {
                let literal = cx.literal_source(*s, self.span).ok_or(fmt::Error)?;
                write!(f, "{}", literal)?;
            }
            Kind::Label(s) => {
                let literal = cx.literal_source(*s, self.span).ok_or(fmt::Error)?;
                write!(f, "'{}", literal)?;
            }
            Kind::Byte(s) => match s {
                CopySource::Text(source_id) => {
                    let s = cx
                        .idx
                        .q
                        .sources
                        .source(*source_id, self.span)
                        .ok_or(fmt::Error)?;
                    write!(f, "{}", s)?;
                }
                CopySource::Inline(b) => {
                    write!(f, "{:?}", b)?;
                }
            },
            Kind::ByteStr(s) => match s {
                StrSource::Text(text) => {
                    let span = if text.wrapped {
                        self.span.narrow(1u32)
                    } else {
                        self.span
                    };

                    let s = cx
                        .idx
                        .q
                        .sources
                        .source(text.source_id, span)
                        .ok_or(fmt::Error)?;

                    write!(f, "b\"{}\"", s)?;
                }
                StrSource::Synthetic(id) => {
                    let b = cx.idx.q.storage.get_byte_string(*id).ok_or(fmt::Error)?;
                    write!(f, "{}", FormatBytes(b))?;
                }
            },
            Kind::Str(s) => match s {
                StrSource::Text(text) => {
                    let span = if text.wrapped {
                        self.span.narrow(1u32)
                    } else {
                        self.span
                    };

                    let s = cx
                        .idx
                        .q
                        .sources
                        .source(text.source_id, span)
                        .ok_or(fmt::Error)?;
                    write!(f, "\"{}\"", s)?;
                }
                StrSource::Synthetic(id) => {
                    let s = cx.idx.q.storage.get_string(*id).ok_or(fmt::Error)?;
                    write!(f, "{:?}", s)?;
                }
            },
            Kind::Char(s) => match s {
                CopySource::Text(source_id) => {
                    let s = cx
                        .idx
                        .q
                        .sources
                        .source(*source_id, self.span)
                        .ok_or(fmt::Error)?;
                    write!(f, "{}", s)?;
                }
                CopySource::Inline(c) => {
                    write!(f, "{:?}", c)?;
                }
            },
            Kind::Number(s) => match s {
                NumberSource::Text(text) => {
                    let s = cx
                        .idx
                        .q
                        .sources
                        .source(text.source_id, self.span)
                        .ok_or(fmt::Error)?;
                    write!(f, "{}", s)?;
                }
                NumberSource::Synthetic(id) => {
                    let n = cx.idx.q.storage.get_number(*id).ok_or(fmt::Error)?;
                    write!(f, "{}", n)?;
                }
            },
            other => {
                let s = other.as_literal_str().ok_or(fmt::Error)?;
                write!(f, "{}", s)?;
            }
        }

        return Ok(());

        struct FormatBytes<'a>(&'a [u8]);

        impl fmt::Display for FormatBytes<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "b\"")?;

                for b in bytes_escape_default(self.0) {
                    write!(f, "{}", b as char)?;
                }

                write!(f, "\"")?;
                Ok(())
            }
        }

        fn bytes_escape_default(bytes: &[u8]) -> impl Iterator<Item = u8> + '_ {
            bytes.iter().copied().flat_map(ascii::escape_default)
        }
    }
}

impl Parse for Token {
    fn parse(p: &mut Parser) -> compile::Result<Self> {
        p.next()
    }
}

impl Peek for Token {
    fn peek(p: &mut super::prelude::Peeker<'_>) -> bool {
        !p.is_eof()
    }
}

impl ToTokens for Token {
    fn to_tokens(
        &self,
        _: &mut MacroContext<'_, '_, '_>,
        stream: &mut TokenStream,
    ) -> alloc::Result<()> {
        stream.push(*self)
    }
}

impl Spanned for Token {
    fn span(&self) -> Span {
        self.span
    }
}

impl IntoExpectation for Token {
    fn into_expectation(self) -> Expectation {
        self.kind.into_expectation()
    }
}

/// The value of a number literal.
#[derive(Debug, TryClone)]
#[non_exhaustive]
pub enum NumberValue {
    /// A float literal number.
    Float(f64),
    /// An integer literal number.
    Integer(#[try_clone(with = num::BigInt::clone)] num::BigInt),
}

/// The suffix of a number.
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub enum NumberSuffix {
    /// The `i64` suffix.
    Int(Span),
    /// The `f64` suffix.
    Float(Span),
    /// The `u8` suffix.
    Byte(Span),
}

/// A resolved number literal.
#[derive(Debug, TryClone)]
#[non_exhaustive]
pub struct Number {
    /// The parsed number value.
    pub value: NumberValue,
    /// The parsed number suffix.
    pub suffix: Option<NumberSuffix>,
}

impl Number {
    /// Convert into a 32-bit unsigned number.
    pub(crate) fn as_u32(&self, neg: bool) -> Option<u32> {
        self.as_primitive(neg, num::ToPrimitive::to_u32)
    }

    /// Convert into usize.
    pub(crate) fn as_usize(&self, neg: bool) -> Option<usize> {
        self.as_primitive(neg, num::ToPrimitive::to_usize)
    }

    /// Try to convert number into a tuple index.
    pub(crate) fn as_tuple_index(&self) -> Option<usize> {
        use num::ToPrimitive;

        match &self.value {
            NumberValue::Integer(n) => n.to_usize(),
            _ => None,
        }
    }

    fn as_primitive<T>(&self, neg: bool, to: impl FnOnce(&num::BigInt) -> Option<T>) -> Option<T> {
        let NumberValue::Integer(number) = &self.value else {
            return None;
        };

        let mut number = number;
        let negated;

        if neg {
            negated = number.clone().neg();
            number = &negated;
        }

        to(number)
    }
}

macro_rules! impl_from_int {
    ($ty:ty) => {
        impl From<$ty> for Number {
            #[inline]
            fn from(value: $ty) -> Self {
                Self {
                    value: NumberValue::Integer(num::BigInt::from(value)),
                    suffix: None,
                }
            }
        }
    };
}

impl_from_int!(usize);
impl_from_int!(isize);
impl_from_int!(i16);
impl_from_int!(u16);
impl_from_int!(i32);
impl_from_int!(u32);
impl_from_int!(i64);
impl_from_int!(u64);
impl_from_int!(i128);
impl_from_int!(u128);

impl From<f32> for Number {
    #[inline]
    fn from(value: f32) -> Self {
        Self {
            value: NumberValue::Float(value as f64),
            suffix: None,
        }
    }
}

impl From<f64> for Number {
    #[inline]
    fn from(value: f64) -> Self {
        Self {
            value: NumberValue::Float(value),
            suffix: None,
        }
    }
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.value {
            NumberValue::Float(n) => write!(f, "{}", n),
            NumberValue::Integer(n) => write!(f, "{}", n),
        }
    }
}

/// The kind of a number literal.
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub enum NumberBase {
    /// A decimal number literal, like `3.14`.
    Decimal,
    /// A hex literal, like `0xffff`.
    Hex,
    /// An octal literal, like `0o7711`.
    Octal,
    /// A binary literal, like `0b110011`.
    Binary,
}

impl fmt::Display for NumberBase {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Decimal => write!(fmt, "decimal"),
            Self::Hex => write!(fmt, "hex"),
            Self::Octal => write!(fmt, "octal"),
            Self::Binary => write!(fmt, "binary"),
        }
    }
}

/// A built-in identifiers that do not have a source.
///
/// This is necessary to synthesize identifiers in the lexer since there's not
/// storage available, nor is the identifier reflected in the source.
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub enum BuiltIn {
    /// `template`.
    Template,
    /// `formatspec`.
    Format,
    /// `builtin`.
    BuiltIn,
    /// `literal`.
    Literal,
    /// `doc`.
    Doc,
}

impl BuiltIn {
    /// Coerce into static string.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Template => "template",
            Self::Format => "formatspec",
            Self::BuiltIn => "builtin",
            Self::Literal => "literal",
            Self::Doc => "doc",
        }
    }
}

impl fmt::Display for BuiltIn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The kind of the identifier.
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub enum LitSource {
    /// The identifier is from the source text.
    Text(SourceId),
    /// The identifier is synthetic (generated in a macro).
    Synthetic(SyntheticId),
    /// Built-in strings.
    BuiltIn(BuiltIn),
}

/// The source of the literal string. This need to be treated separately from
/// [LitSource] because it might encompass special things like quoting and
/// escaping.
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub enum StrSource {
    /// The literal string source is from the source text.
    Text(StrText),
    /// The string source is synthetic (generated in a macro).
    Synthetic(SyntheticId),
}

/// Configuration for a literal string.
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct StrText {
    /// The source of the text.
    pub source_id: SourceId,
    /// Indicates if the string is escaped or not.
    pub escaped: bool,
    /// Indicated if the buffer is wrapped or not.
    pub wrapped: bool,
}

/// The source of a number.
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub enum NumberSource {
    /// The number is from the source text (and need to be parsed while it's
    /// being resolved).
    Text(NumberText),
    /// The number is synthetic, and stored in the specified slot.
    Synthetic(SyntheticId),
}

/// The source of an item that implements Copy.
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
#[try_clone(bound = {T: TryClone})]
pub enum CopySource<T>
where
    T: Copy,
{
    /// The item is from the source text (and need to be parsed while it's being
    /// resolved).
    Text(SourceId),
    /// The char is inlined in the ast.
    Inline(T),
}

/// Configuration of a text number.
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct NumberText {
    /// The source of the text.
    pub source_id: SourceId,
    /// Indicates if it's a decimal number.
    pub is_fractional: bool,
    /// The number literal kind.
    pub base: NumberBase,
    /// The number part of the parsed number.
    pub number: Span,
    /// The suffix.
    pub suffix: Span,
}

/// A delimiter, `{`, `{`, or `[`.
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[try_clone(copy)]
#[non_exhaustive]
pub enum Delimiter {
    /// A parenthesis delimiter `(` and `)`.
    Parenthesis,
    /// A brace delimiter `{` and `}`.
    Brace,
    /// A bracket delimiter `[` and `]`.
    Bracket,
    /// An empty group delimiter.
    Empty,
}

impl Delimiter {
    /// The character used as an open delimiter.
    pub(crate) fn open(self) -> &'static str {
        match self {
            Self::Parenthesis => "(",
            Self::Brace => "{",
            Self::Bracket => "[",
            Self::Empty => "",
        }
    }

    /// The character used as a close delimiter.
    pub(crate) fn close(self) -> &'static str {
        match self {
            Self::Parenthesis => ")",
            Self::Brace => "}",
            Self::Bracket => "]",
            Self::Empty => "",
        }
    }
}
