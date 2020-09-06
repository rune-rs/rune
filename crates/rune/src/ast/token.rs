use runestick::Span;
use std::fmt;

/// A single token encountered during parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Token {
    /// The span of the token.
    pub span: Span,
    /// The kind of the token.
    pub kind: Kind,
}

/// The kind of a number literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NumberKind {
    /// A decimal number literal, like `3.14`.
    Decimal,
    /// A hex literal, like `0xffff`.
    Hex,
    /// An octal literal, like `0o7711`.
    Octal,
    /// A binary literal, like `0b110011`.
    Binary,
}

impl fmt::Display for NumberKind {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Decimal => write!(fmt, "decimal"),
            Self::Hex => write!(fmt, "hex"),
            Self::Octal => write!(fmt, "octal"),
            Self::Binary => write!(fmt, "binary"),
        }
    }
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
    pub fn open(self) -> char {
        match self {
            Self::Parenthesis => '(',
            Self::Brace => '{',
            Self::Bracket => '[',
        }
    }

    /// The character used as a close delimiter.
    pub fn close(self) -> char {
        match self {
            Self::Parenthesis => ')',
            Self::Brace => '}',
            Self::Bracket => ']',
        }
    }
}

/// The kind of the token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    /// A `self` token.
    Self_,
    /// An `fn` token.
    Fn,
    /// An `enum` token.
    Enum,
    /// A `struct` token.
    Struct,
    /// An `is` token.
    Is,
    /// An `not` token.
    Not,
    /// A `let` token.
    Let,
    /// An `if` token.
    If,
    /// A `match` token.
    Match,
    /// An `else` token.
    Else,
    /// An `use` token.
    Use,
    /// A `while` token.
    While,
    /// A `loop` token.
    Loop,
    /// A `for` token.
    For,
    /// An `in` token.
    In,
    /// The `true` literal.
    True,
    /// The `false` literal.
    False,
    /// A `break` token.
    Break,
    /// A `yield` token.
    Yield,
    /// A `return` token.
    Return,
    /// The `await` keyword.
    Await,
    /// The `async` keyword.
    Async,
    /// The `select` keyword.
    Select,
    /// The `default` keyword.
    Default,
    /// The `impl` keyword.
    Impl,
    /// The `mod` keyword.
    Mod,
    /// An identifier.
    Ident,
    /// A label, like `'loop`.
    Label,
    /// A number literal, like `42` or `3.14` or `0xff`.
    LitNumber {
        /// Indicates if it's a decimal number.
        is_fractional: bool,
        /// Indicates if the number is negative.
        is_negative: bool,
        /// The number literal kind.
        number: NumberKind,
    },
    /// A characer literal.
    LitChar,
    /// A byte literal.
    LitByte,
    /// A string literal, including escape sequences. Like `"hello\nworld"`.
    LitStr {
        /// If the string literal contains escapes.
        escaped: bool,
    },
    /// A byte string literal, including escape sequences. Like `b"hello\nworld"`.
    LitByteStr {
        /// If the string literal contains escapes.
        escaped: bool,
    },
    /// A template literal, including escape sequences. Like ``hello {name}``.
    LitTemplate {
        /// If the template contains escapes.
        escaped: bool,
    },
    /// An open delimiter: `(`, `{`, or `[`.
    Open(Delimiter),
    /// A close delimiter: `)`, `}`, or `]`.
    Close(Delimiter),
    /// `#`.
    Hash,
    /// `.`.
    Dot,
    /// `::`.
    ColonColon,
    /// `_`.
    Underscore,
    /// `,`.
    Comma,
    /// `:`.
    Colon,
    /// `;`.
    SemiColon,
    /// `+`.
    Plus,
    /// `-`.
    Dash,
    /// `/`.
    Div,
    /// `*`.
    Star,
    /// `&`.
    Amp,
    /// `=`.
    Eq,
    /// `==`.
    EqEq,
    /// `!=`.
    BangEq,
    /// `=>`.
    Rocket,
    /// `<`.
    Lt,
    /// `>`.
    Gt,
    /// `<=`.
    LtEq,
    /// `>=`.
    GtEq,
    /// `!`.
    Bang,
    /// `?`.
    QuestionMark,
    /// `..`.
    DotDot,
    /// `&&`.
    AmpAmp,
    /// `||`.
    PipePipe,
    /// `|`.
    Pipe,
    /// `%`.
    Perc,
    /// `<<`.
    LtLt,
    /// `>>`.
    GtGt,
    /// `^`.
    Caret,
    /// `+=`.
    PlusEq,
    /// `-=`.
    DashEq,
    /// `*=`.
    StarEq,
    /// `/=`.
    SlashEq,
    /// `%=`.
    PercEq,
    /// `&=`.
    AmpEq,
    /// `^=`.
    CaretEq,
    /// |=`.
    PipeEq,
    /// `<<=`.
    LtLtEq,
    /// `>>=`.
    GtGtEq,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Self_ => write!(f, "self")?,
            Self::Fn => write!(f, "fn")?,
            Self::Enum => write!(f, "enum")?,
            Self::Struct => write!(f, "struct")?,
            Self::Is => write!(f, "is")?,
            Self::Not => write!(f, "not")?,
            Self::Let => write!(f, "let")?,
            Self::If => write!(f, "if")?,
            Self::Match => write!(f, "match")?,
            Self::Else => write!(f, "else")?,
            Self::Use => write!(f, "use")?,
            Self::While => write!(f, "while")?,
            Self::Loop => write!(f, "loop")?,
            Self::For => write!(f, "for")?,
            Self::In => write!(f, "in")?,
            Self::True => write!(f, "true")?,
            Self::False => write!(f, "false")?,
            Self::Break => write!(f, "break")?,
            Self::Yield => write!(f, "yield")?,
            Self::Return => write!(f, "return")?,
            Self::Await => write!(f, "await")?,
            Self::Async => write!(f, "async")?,
            Self::Select => write!(f, "select")?,
            Self::Default => write!(f, "default")?,
            Self::Impl => write!(f, "impl")?,
            Self::Mod => write!(f, "mod")?,
            Self::Ident => write!(f, "ident")?,
            Self::Label => write!(f, "label")?,
            Self::LitNumber { .. } => write!(f, "number")?,
            Self::LitStr { .. } => write!(f, "string")?,
            Self::LitByteStr { .. } => write!(f, "byte string")?,
            Self::LitTemplate { .. } => write!(f, "template")?,
            Self::LitChar { .. } => write!(f, "char")?,
            Self::LitByte { .. } => write!(f, "byte")?,
            Self::Open(delimiter) => write!(f, "{}", delimiter.open())?,
            Self::Close(delimiter) => write!(f, "{}", delimiter.close())?,
            Self::Underscore => write!(f, "_")?,
            Self::Comma => write!(f, ",")?,
            Self::Colon => write!(f, ":")?,
            Self::Hash => write!(f, "#")?,
            Self::Dot => write!(f, ".")?,
            Self::ColonColon => write!(f, "::")?,
            Self::SemiColon => write!(f, ";")?,
            Self::Caret => write!(f, "^")?,
            Self::Plus => write!(f, "+")?,
            Self::Dash => write!(f, "-")?,
            Self::Div => write!(f, "/")?,
            Self::PlusEq => write!(f, "+=")?,
            Self::DashEq => write!(f, "-=")?,
            Self::StarEq => write!(f, "*=")?,
            Self::SlashEq => write!(f, "/=")?,
            Self::PercEq => write!(f, "%=")?,
            Self::AmpEq => write!(f, "&=")?,
            Self::CaretEq => write!(f, "^=")?,
            Self::PipeEq => write!(f, "|=")?,
            Self::LtLt => write!(f, "<<")?,
            Self::GtGt => write!(f, ">>")?,
            Self::LtLtEq => write!(f, "<<=")?,
            Self::GtGtEq => write!(f, ">>=")?,
            Self::Star => write!(f, "*")?,
            Self::Amp => write!(f, "&")?,
            Self::Eq => write!(f, "=")?,
            Self::EqEq => write!(f, "==")?,
            Self::BangEq => write!(f, "!=")?,
            Self::Rocket => write!(f, "=>")?,
            Self::Lt => write!(f, "<")?,
            Self::Gt => write!(f, ">")?,
            Self::LtEq => write!(f, "<=")?,
            Self::GtEq => write!(f, ">=")?,
            Self::Bang => write!(f, "!")?,
            Self::QuestionMark => write!(f, "?")?,
            Self::DotDot => write!(f, "..")?,
            Self::AmpAmp => write!(f, "&&")?,
            Self::PipePipe => write!(f, "||")?,
            Self::Pipe => write!(f, "|")?,
            Self::Perc => write!(f, "%")?,
        }

        Ok(())
    }
}
