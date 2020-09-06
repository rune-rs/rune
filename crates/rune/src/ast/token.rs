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
    /// A hash `#`.
    Hash,
    /// A dot `.`.
    Dot,
    /// A scope `::`.
    Scope,
    /// An underscore `_`.
    Underscore,
    /// A comma `,`.
    Comma,
    /// A colon `:`.
    Colon,
    /// A semi-colon `;`.
    SemiColon,
    /// An add operator `+`.
    Add,
    /// An add assign operator `+=`.
    AddAssign,
    /// A sub operator `-`.
    Sub,
    /// An sub assign operator `-=`.
    SubAssign,
    /// A division operator `/`.
    Div,
    /// An division assign operator `/=`.
    DivAssign,
    /// A multiply operator `*`.
    Mul,
    /// An multiply assign operator `*=`.
    MulAssign,
    /// An ampersand literal `&`.
    Ampersand,
    /// An equals sign `=`.
    Eq,
    /// Two equals sign `==`.
    EqEq,
    /// Not equals `!=`.
    Neq,
    /// The rocket token `=>`.
    Rocket,
    /// Less than comparison `<`.
    Lt,
    /// Greater than comparison `>`.
    Gt,
    /// Less than or equal comparison `<=`.
    Lte,
    /// Greater than or equal comparison `>=`.
    Gte,
    /// Bang operator `!`.
    Bang,
    /// Try operator `?`.
    Try,
    /// Double dots `..`.
    DotDot,
    /// And operator.
    And,
    /// Or operator.
    Or,
    /// A `|` pipe.
    Pipe,
    /// A `%` operator.
    Rem,
}

impl fmt::Display for Kind {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Self_ => write!(fmt, "self")?,
            Self::Fn => write!(fmt, "fn")?,
            Self::Enum => write!(fmt, "enum")?,
            Self::Struct => write!(fmt, "struct")?,
            Self::Is => write!(fmt, "is")?,
            Self::Not => write!(fmt, "not")?,
            Self::Let => write!(fmt, "let")?,
            Self::If => write!(fmt, "if")?,
            Self::Match => write!(fmt, "match")?,
            Self::Else => write!(fmt, "else")?,
            Self::Use => write!(fmt, "use")?,
            Self::While => write!(fmt, "while")?,
            Self::Loop => write!(fmt, "loop")?,
            Self::For => write!(fmt, "for")?,
            Self::In => write!(fmt, "in")?,
            Self::True => write!(fmt, "true")?,
            Self::False => write!(fmt, "false")?,
            Self::Break => write!(fmt, "break")?,
            Self::Yield => write!(fmt, "yield")?,
            Self::Return => write!(fmt, "return")?,
            Self::Await => write!(fmt, "await")?,
            Self::Async => write!(fmt, "async")?,
            Self::Select => write!(fmt, "select")?,
            Self::Default => write!(fmt, "default")?,
            Self::Impl => write!(fmt, "impl")?,
            Self::Mod => write!(fmt, "mod")?,
            Self::Ident => write!(fmt, "ident")?,
            Self::Label => write!(fmt, "label")?,
            Self::LitNumber { .. } => write!(fmt, "number")?,
            Self::LitStr { .. } => write!(fmt, "string")?,
            Self::LitByteStr { .. } => write!(fmt, "byte string")?,
            Self::LitTemplate { .. } => write!(fmt, "template")?,
            Self::LitChar { .. } => write!(fmt, "char")?,
            Self::LitByte { .. } => write!(fmt, "byte")?,
            Self::Open(delimiter) => write!(fmt, "{}", delimiter.open())?,
            Self::Close(delimiter) => write!(fmt, "{}", delimiter.close())?,
            Self::Underscore => write!(fmt, "_")?,
            Self::Comma => write!(fmt, ",")?,
            Self::Colon => write!(fmt, ":")?,
            Self::Hash => write!(fmt, "#")?,
            Self::Dot => write!(fmt, ".")?,
            Self::Scope => write!(fmt, "::")?,
            Self::SemiColon => write!(fmt, ";")?,
            Self::Add => write!(fmt, "+")?,
            Self::AddAssign => write!(fmt, "+=")?,
            Self::Sub => write!(fmt, "-")?,
            Self::SubAssign => write!(fmt, "-=")?,
            Self::Div => write!(fmt, "/")?,
            Self::DivAssign => write!(fmt, "/=")?,
            Self::Mul => write!(fmt, "*")?,
            Self::MulAssign => write!(fmt, "*=")?,
            Self::Ampersand => write!(fmt, "&")?,
            Self::Eq => write!(fmt, "=")?,
            Self::EqEq => write!(fmt, "==")?,
            Self::Neq => write!(fmt, "!=")?,
            Self::Rocket => write!(fmt, "=>")?,
            Self::Lt => write!(fmt, "<")?,
            Self::Gt => write!(fmt, ">")?,
            Self::Lte => write!(fmt, "<=")?,
            Self::Gte => write!(fmt, ">=")?,
            Self::Bang => write!(fmt, "!")?,
            Self::Try => write!(fmt, "?")?,
            Self::DotDot => write!(fmt, "..")?,
            Self::And => write!(fmt, "&&")?,
            Self::Or => write!(fmt, "||")?,
            Self::Pipe => write!(fmt, "|")?,
            Self::Rem => write!(fmt, "%")?,
        }

        Ok(())
    }
}
