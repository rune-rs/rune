use crate::ast::Kind;
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
            Kind::Abstract => {
                write!(f, "abstract")?;
            }
            Kind::AlignOf => {
                write!(f, "alignof")?;
            }
            Kind::Amp => {
                write!(f, "&")?;
            }
            Kind::AmpAmp => {
                write!(f, "&&")?;
            }
            Kind::AmpEq => {
                write!(f, "&=")?;
            }
            Kind::Arrow => {
                write!(f, "=>")?;
            }
            Kind::As => {
                write!(f, "as")?;
            }
            Kind::Async => {
                write!(f, "async")?;
            }
            Kind::At => {
                write!(f, "at")?;
            }
            Kind::Await => {
                write!(f, "await")?;
            }
            Kind::Bang => {
                write!(f, "!")?;
            }
            Kind::BangEq => {
                write!(f, "!=")?;
            }
            Kind::Become => {
                write!(f, "become")?;
            }
            Kind::Break => {
                write!(f, "break")?;
            }
            Kind::Caret => {
                write!(f, "^")?;
            }
            Kind::CaretEq => {
                write!(f, "^=")?;
            }
            Kind::Close(d) => {
                write!(f, "{}", d.close())?;
            }
            Kind::Colon => {
                write!(f, ":")?;
            }
            Kind::ColonColon => {
                write!(f, "::")?;
            }
            Kind::Comma => {
                write!(f, ",")?;
            }
            Kind::Const => {
                write!(f, "const")?;
            }
            Kind::Crate => {
                write!(f, "crate")?;
            }
            Kind::Dash => {
                write!(f, "-")?;
            }
            Kind::DashEq => {
                write!(f, "-=")?;
            }
            Kind::Default => {
                write!(f, "default")?;
            }
            Kind::Div => {
                write!(f, "/")?;
            }
            Kind::Do => {
                write!(f, "do")?;
            }
            Kind::Dollar => {
                write!(f, "$")?;
            }
            Kind::Dot => {
                write!(f, ".")?;
            }
            Kind::DotDot => {
                write!(f, "..")?;
            }
            Kind::Else => {
                write!(f, "else")?;
            }
            Kind::Enum => {
                write!(f, "enum")?;
            }
            Kind::Eq => {
                write!(f, "=")?;
            }
            Kind::EqEq => {
                write!(f, "==")?;
            }
            Kind::Extern => {
                write!(f, "extern")?;
            }
            Kind::False => {
                write!(f, "false")?;
            }
            Kind::Final => {
                write!(f, "final")?;
            }
            Kind::Fn => {
                write!(f, "fn")?;
            }
            Kind::For => {
                write!(f, "for")?;
            }
            Kind::Gt => {
                write!(f, ">")?;
            }
            Kind::GtEq => {
                write!(f, ">=")?;
            }
            Kind::GtGt => {
                write!(f, ">>")?;
            }
            Kind::GtGtEq => {
                write!(f, ">>=")?;
            }
            Kind::Ident(s) => match s {
                StringSource::Text => {
                    let s = ctx.source().source(self.span).ok_or_else(|| fmt::Error)?;
                    write!(f, "{}", s)?;
                }
                StringSource::Synthetic(id) => {
                    match ctx.storage().with_string(*id, |s| write!(f, "{:?}", s)) {
                        Some(result) => result?,
                        None => return Err(fmt::Error),
                    }
                }
            },
            Kind::If => {
                write!(f, "if")?;
            }
            Kind::Impl => {
                write!(f, "impl")?;
            }
            Kind::In => {
                write!(f, "in")?;
            }
            Kind::Is => {
                write!(f, "is")?;
            }
            Kind::Label(s) => match s {
                StringSource::Text => {
                    let s = ctx.source().source(self.span).ok_or_else(|| fmt::Error)?;
                    write!(f, "{}", s)?;
                }
                StringSource::Synthetic(id) => {
                    match ctx.storage().with_string(*id, |s| write!(f, "'{}", s)) {
                        Some(result) => result?,
                        None => return Err(fmt::Error),
                    }
                }
            },
            Kind::Let => {
                write!(f, "let")?;
            }
            Kind::LitByte(s) => match s {
                CopySource::Text => {
                    let s = ctx.source().source(self.span).ok_or_else(|| fmt::Error)?;
                    write!(f, "{}", s)?;
                }
                CopySource::Inline(b) => {
                    write!(f, "{:?}", b)?;
                }
            },
            Kind::LitByteStr(s) => match s {
                LitStrSource::Text(text) => {
                    let span = if text.wrapped {
                        self.span.narrow(1)
                    } else {
                        self.span
                    };

                    let s = ctx.source().source(span).ok_or_else(|| fmt::Error)?;
                    write!(f, "b\"{}\"", s)?;
                }
                LitStrSource::Synthetic(id) => {
                    match ctx
                        .storage()
                        .with_byte_string(*id, |bytes| write!(f, "{}", FormatBytes(bytes)))
                    {
                        Some(result) => result?,
                        None => return Err(fmt::Error),
                    }
                }
            },
            Kind::LitChar(s) => match s {
                CopySource::Text => {
                    let s = ctx.source().source(self.span).ok_or_else(|| fmt::Error)?;
                    write!(f, "{}", s)?;
                }
                CopySource::Inline(c) => {
                    write!(f, "{:?}", c)?;
                }
            },
            Kind::LitNumber(s) => match s {
                NumberSource::Text(_) => {
                    let s = ctx.source().source(self.span).ok_or_else(|| fmt::Error)?;
                    write!(f, "{}", s)?;
                }
                NumberSource::Synthetic(id) => {
                    match ctx.storage().with_number(*id, |n| write!(f, "{}", n)) {
                        Some(result) => result?,
                        None => return Err(fmt::Error),
                    }
                }
            },
            Kind::LitStr(s) => match s {
                LitStrSource::Text(text) => {
                    let span = if text.wrapped {
                        self.span.narrow(1)
                    } else {
                        self.span
                    };

                    let s = ctx.source().source(span).ok_or_else(|| fmt::Error)?;
                    write!(f, "\"{}\"", s)?;
                }
                LitStrSource::Synthetic(id) => {
                    match ctx.storage().with_string(*id, |s| write!(f, "{:?}", s)) {
                        Some(result) => result?,
                        None => return Err(fmt::Error),
                    }
                }
            },
            Kind::Loop => {
                write!(f, "loop")?;
            }
            Kind::Lt => {
                write!(f, "<")?;
            }
            Kind::LtEq => {
                write!(f, "<=")?;
            }
            Kind::LtLt => {
                write!(f, "<<")?;
            }
            Kind::LtLtEq => {
                write!(f, "<<=")?;
            }
            Kind::Macro => {
                write!(f, "macro")?;
            }
            Kind::Match => {
                write!(f, "match")?;
            }
            Kind::Mod => {
                write!(f, "mod")?;
            }
            Kind::Move => {
                write!(f, "move")?;
            }
            Kind::Not => {
                write!(f, "not")?;
            }
            Kind::OffsetOf => {
                write!(f, "offsetof")?;
            }
            Kind::Open(d) => {
                write!(f, "{}", d.open())?;
            }
            Kind::Override => {
                write!(f, "override")?;
            }
            Kind::Perc => {
                write!(f, "%")?;
            }
            Kind::PercEq => {
                write!(f, "%=")?;
            }
            Kind::Pipe => {
                write!(f, "|")?;
            }
            Kind::PipeEq => {
                write!(f, "|=")?;
            }
            Kind::PipePipe => {
                write!(f, "||")?;
            }
            Kind::Plus => {
                write!(f, "+")?;
            }
            Kind::PlusEq => {
                write!(f, "+=")?;
            }
            Kind::Pound => {
                write!(f, "#")?;
            }
            Kind::Priv => {
                write!(f, "priv")?;
            }
            Kind::Proc => {
                write!(f, "proc")?;
            }
            Kind::Pub => {
                write!(f, "pub")?;
            }
            Kind::Pure => {
                write!(f, "pure")?;
            }
            Kind::QuestionMark => {
                write!(f, "?")?;
            }
            Kind::Ref => {
                write!(f, "ref")?;
            }
            Kind::Return => {
                write!(f, "return")?;
            }
            Kind::Rocket => {
                write!(f, "=>")?;
            }
            Kind::Select => {
                write!(f, "select")?;
            }
            Kind::SelfType => {
                write!(f, "Self")?;
            }
            Kind::SelfValue => {
                write!(f, "self")?;
            }
            Kind::SemiColon => {
                write!(f, ";")?;
            }
            Kind::SizeOf => {
                write!(f, "sizeof")?;
            }
            Kind::SlashEq => {
                write!(f, "/=")?;
            }
            Kind::Star => {
                write!(f, "*")?;
            }
            Kind::StarEq => {
                write!(f, "*=")?;
            }
            Kind::Static => {
                write!(f, "static")?;
            }
            Kind::Struct => {
                write!(f, "struct")?;
            }
            Kind::Super => {
                write!(f, "super")?;
            }
            Kind::Template => {
                write!(f, "template")?;
            }
            Kind::Tilde => {
                write!(f, "~")?;
            }
            Kind::True => {
                write!(f, "true")?;
            }
            Kind::TypeOf => {
                write!(f, "typeof")?;
            }
            Kind::Underscore => {
                write!(f, "_")?;
            }
            Kind::Unsafe => {
                write!(f, "unsafe")?;
            }
            Kind::Use => {
                write!(f, "use")?;
            }
            Kind::Virtual => {
                write!(f, "virtual")?;
            }
            Kind::While => {
                write!(f, "while")?;
            }
            Kind::Yield => {
                write!(f, "yield")?;
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

/// A resolved number literal.
#[derive(Debug, Clone)]
pub enum Number {
    /// A float literal number.
    Float(f64),
    /// An integer literal number.
    Integer(num::BigInt),
}

impl Number {
    /// Convert into a 64-bit signed number.
    pub fn as_i64(&self, spanned: Span, neg: bool) -> Result<i64, ParseError> {
        use num::ToPrimitive as _;
        use std::ops::Neg as _;

        let number = match self {
            Number::Float(_) => return Err(ParseError::new(spanned, ParseErrorKind::BadNumber)),
            Number::Integer(n) => {
                if neg {
                    n.clone().neg().to_i64()
                } else {
                    n.to_i64()
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

impl From<f64> for Number {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<u32> for Number {
    fn from(value: u32) -> Self {
        Self::Integer(num::BigInt::from(value))
    }
}

impl From<i32> for Number {
    fn from(value: i32) -> Self {
        Self::Integer(num::BigInt::from(value))
    }
}

impl From<u64> for Number {
    fn from(value: u64) -> Self {
        Self::Integer(num::BigInt::from(value))
    }
}

impl From<i64> for Number {
    fn from(value: i64) -> Self {
        Self::Integer(num::BigInt::from(value))
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

/// The kind of the identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StringSource {
    /// The identifier is from the source text.
    Text,
    /// The identifier is synthetic (generated in a macro).
    Synthetic(usize),
}

/// The source of the literal string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LitStrSource {
    /// The literal string source is from the source text.
    Text(LitStrSourceText),
    /// The string source is synthetic (generated in a macro).
    Synthetic(usize),
}

/// Configuration for a literal string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LitStrSourceText {
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
    Text(NumberSourceText),
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
pub struct NumberSourceText {
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
