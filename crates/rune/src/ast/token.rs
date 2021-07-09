use crate::ast::Kind;
use crate::shared::Description;
use crate::{MacroContext, ParseError, ParseErrorKind, Spanned};
use runestick::Span;
use std::fmt;

/// A single token encountered during parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    /// The span of the token.
    pub span: Span,
    /// The kind of the token.
    pub kind: Kind,
}

impl Token {
    /// Format the current token to a formatter.
    pub fn token_fmt(&self, ctx: &MacroContext, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            Kind::Eof | Kind::Error => {
                // NB: marker tokens can't be formatted.
                return Err(fmt::Error);
            }
            Kind::Ident(s) => match s {
                StringSource::Text => {
                    let s = ctx.source().source(self.span).ok_or(fmt::Error)?;
                    write!(f, "{}", s)?;
                }
                StringSource::Synthetic(id) => {
                    match ctx.storage().with_string(*id, |s| write!(f, "{}", s)) {
                        Some(result) => result?,
                        None => return Err(fmt::Error),
                    }
                }
                StringSource::BuiltIn(builtin) => {
                    write!(f, "{}", builtin)?;
                }
            },
            Kind::Label(s) => match s {
                StringSource::Text => {
                    let s = ctx.source().source(self.span).ok_or(fmt::Error)?;
                    write!(f, "{}", s)?;
                }
                StringSource::Synthetic(id) => {
                    match ctx.storage().with_string(*id, |s| write!(f, "'{}", s)) {
                        Some(result) => result?,
                        None => return Err(fmt::Error),
                    }
                }
                StringSource::BuiltIn(builtin) => {
                    write!(f, "'{}", builtin)?;
                }
            },
            Kind::Byte(s) => match s {
                CopySource::Text => {
                    let s = ctx.source().source(self.span).ok_or(fmt::Error)?;
                    write!(f, "{}", s)?;
                }
                CopySource::Inline(b) => {
                    write!(f, "{:?}", b)?;
                }
            },
            Kind::ByteStr(s) => match s {
                StrSource::Text(text) => {
                    let span = if text.wrapped {
                        self.span.narrow(1)
                    } else {
                        self.span
                    };

                    let s = ctx.source().source(span).ok_or(fmt::Error)?;
                    write!(f, "b\"{}\"", s)?;
                }
                StrSource::Synthetic(id) => {
                    match ctx
                        .storage()
                        .with_byte_string(*id, |bytes| write!(f, "{}", FormatBytes(bytes)))
                    {
                        Some(result) => result?,
                        None => return Err(fmt::Error),
                    }
                }
            },
            Kind::Char(s) => match s {
                CopySource::Text => {
                    let s = ctx.source().source(self.span).ok_or(fmt::Error)?;
                    write!(f, "{}", s)?;
                }
                CopySource::Inline(c) => {
                    write!(f, "{:?}", c)?;
                }
            },
            Kind::Number(s) => match s {
                NumberSource::Text(_) => {
                    let s = ctx.source().source(self.span).ok_or(fmt::Error)?;
                    write!(f, "{}", s)?;
                }
                NumberSource::Synthetic(id) => {
                    match ctx.storage().with_number(*id, |n| write!(f, "{}", n)) {
                        Some(result) => result?,
                        None => return Err(fmt::Error),
                    }
                }
            },
            Kind::Str(s) => match s {
                StrSource::Text(text) => {
                    let span = if text.wrapped {
                        self.span.narrow(1)
                    } else {
                        self.span
                    };

                    let s = ctx.source().source(span).ok_or(fmt::Error)?;
                    write!(f, "\"{}\"", s)?;
                }
                StrSource::Synthetic(id) => {
                    match ctx.storage().with_string(*id, |s| write!(f, "{:?}", s)) {
                        Some(result) => result?,
                        None => return Err(fmt::Error),
                    }
                }
            },
            other => {
                write!(f, "{}", other)?;
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
            bytes.iter().copied().flat_map(std::ascii::escape_default)
        }
    }
}

impl crate::ToTokens for Token {
    fn to_tokens(&self, _: &MacroContext, stream: &mut crate::TokenStream) {
        stream.push(*self);
    }
}

impl Spanned for Token {
    fn span(&self) -> Span {
        self.span
    }
}

impl Description for &Token {
    fn description(self) -> &'static str {
        self.kind.description()
    }
}

/// A resolved number literal.
#[derive(Debug, Clone)]
pub enum Number {
    /// A float literal number.
    Float(f64),
    /// An integer literal number.
    Integer(num::BigInt),
}

impl Number {
    /// Negate the inner number.
    #[allow(clippy::should_implement_trait)]
    pub fn neg(self) -> Self {
        use std::ops::Neg;

        match self {
            Self::Float(n) => Self::Float(-n),
            Self::Integer(n) => Self::Integer(n.neg()),
        }
    }

    /// Convert into a 32-bit unsigned number.
    pub fn as_u32(&self, spanned: Span, neg: bool) -> Result<u32, ParseError> {
        self.as_primitive(spanned, neg, num::ToPrimitive::to_u32)
    }

    /// Convert into a 64-bit signed number.
    pub fn as_i64(&self, spanned: Span, neg: bool) -> Result<i64, ParseError> {
        self.as_primitive(spanned, neg, num::ToPrimitive::to_i64)
    }

    /// Convert into usize.
    pub fn as_usize(&self, spanned: Span, neg: bool) -> Result<usize, ParseError> {
        self.as_primitive(spanned, neg, num::ToPrimitive::to_usize)
    }

    fn as_primitive<T>(
        &self,
        spanned: Span,
        neg: bool,
        to: impl FnOnce(&num::BigInt) -> Option<T>,
    ) -> Result<T, ParseError> {
        use std::ops::Neg as _;

        let number = match self {
            Number::Float(_) => return Err(ParseError::new(spanned, ParseErrorKind::BadNumber)),
            Number::Integer(n) => {
                if neg {
                    to(&n.clone().neg())
                } else {
                    to(n)
                }
            }
        };

        match number {
            Some(n) => Ok(n),
            None => Err(ParseError::new(
                spanned,
                ParseErrorKind::BadNumberOutOfBounds,
            )),
        }
    }

    /// Try to convert number into a tuple index.
    pub fn as_tuple_index(&self) -> Option<usize> {
        use num::ToPrimitive as _;

        match self {
            Self::Integer(n) => n.to_usize(),
            _ => None,
        }
    }
}

macro_rules! impl_from_int {
    ($ty:ty) => {
        impl From<$ty> for Number {
            fn from(value: $ty) -> Self {
                Self::Integer(num::BigInt::from(value))
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
    fn from(value: f32) -> Self {
        Self::Float(value as f64)
    }
}

impl From<f64> for Number {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Float(n) => write!(f, "{}", n),
            Self::Integer(n) => write!(f, "{}", n),
        }
    }
}

/// The kind of a number literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BuiltIn {
    /// `template`.
    Template,
    /// `formatspec`.
    Format,
    /// `builtin`.
    BuiltIn,
    /// `literal`.
    Literal,
}

impl BuiltIn {
    /// Coerce into static string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Template => "template",
            Self::Format => "formatspec",
            Self::BuiltIn => "builtin",
            Self::Literal => "literal",
        }
    }
}

impl fmt::Display for BuiltIn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The kind of the identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StringSource {
    /// The identifier is from the source text.
    Text,
    /// The identifier is synthetic (generated in a macro).
    Synthetic(usize),
    /// Built-in strings.
    BuiltIn(BuiltIn),
}

/// The source of the literal string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StrSource {
    /// The literal string source is from the source text.
    Text(StrText),
    /// The string source is synthetic (generated in a macro).
    Synthetic(usize),
}

/// Configuration for a literal string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StrText {
    /// Indicates if the string is escaped or not.
    pub escaped: bool,
    /// Indicated if the buffer is wrapped or not.
    pub wrapped: bool,
}

/// The source of a number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NumberSource {
    /// The number is from the source text (and need to be parsed while it's
    /// being resolved).
    Text(NumberText),
    /// The number is synthetic, and stored in the specified slot.
    Synthetic(usize),
}

/// The source of an item that implements Copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CopySource<T>
where
    T: Copy,
{
    /// The item is from the source text (and need to be parsed while it's being
    /// resolved).
    Text,
    /// The char is inlined in the ast.
    Inline(T),
}

/// Configuration of a text number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NumberText {
    /// Indicates if it's a decimal number.
    pub is_fractional: bool,
    /// The number literal kind.
    pub base: NumberBase,
}

/// A delimiter, `{`, `{`, or `[`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Delimiter {
    /// A parenthesis delimiter `(` and `)`.
    Parenthesis,
    /// A brace delimiter `{` and `}`.
    Brace,
    /// A bracket delimiter `[` and `]`.
    Bracket,
}

impl Delimiter {
    /// The character used as an open delimiter.
    pub fn open(self) -> &'static str {
        match self {
            Self::Parenthesis => "(",
            Self::Brace => "{",
            Self::Bracket => "[",
        }
    }

    /// The character used as a close delimiter.
    pub fn close(self) -> &'static str {
        match self {
            Self::Parenthesis => ")",
            Self::Brace => "}",
            Self::Bracket => "]",
        }
    }
}
