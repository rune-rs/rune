use crate::{MacroContext, Spanned};
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

impl crate::IntoTokens for Token {
    fn into_tokens(&self, _: &mut MacroContext, stream: &mut crate::TokenStream) {
        stream.push(*self);
    }
}

impl Spanned for Token {
    fn span(&self) -> Span {
        self.span
    }
}

/// A resolved number literal.
#[derive(Debug, Clone, Copy)]
pub enum Number {
    /// A float literal number.
    Float(f64),
    /// An integer literal number.
    Integer(i64),
}

impl Number {
    /// Try to convert number into a tuple index.
    pub fn into_tuple_index(self) -> Option<usize> {
        use std::convert::TryFrom as _;

        match self {
            Self::Integer(n) if n >= 0 => usize::try_from(n).ok(),
            _ => None,
        }
    }
}

impl From<f64> for Number {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<i64> for Number {
    fn from(value: i64) -> Self {
        Self::Integer(value)
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
}

/// The source of the literal byte string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LitByteStrSource {
    /// The literal source is from the source text.
    Text(LitByteStrSourceText),
    /// The source is synthetic (generated in a macro).
    Synthetic(usize),
}

/// Configuration for a literal byte string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LitByteStrSourceText {
    /// Indicates if the byte string is escaped or not.
    pub escaped: bool,
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
    /// Indicates if the number is negative.
    pub is_negative: bool,
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

macro_rules! kinds {
    ($( $ident:ident $(($ty:ty))?, $doc:literal ),* $(,)?) => {
        /// The kind of the token.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum Kind {
            $(#[doc = $doc] $ident $(($ty))?,)*
        }
    }
}

kinds! {
    Abstract, "The `abstract` keyword.",
    AlignOf, "The `alignof` keyword.",
    Amp, "`&`.",
    AmpAmp, "`&&`.",
    AmpEq, "`&=`.",
    As, "The `as` keyword.",
    Async, "The `async` keyword.",
    At, "`@`.",
    Arrow, "`->`.",
    Await, "The `await` keyword.",
    Bang, "`!`.",
    BangEq, "`!=`.",
    Become, "The `become` keyword.",
    Break, "The `break` keyword.",
    Caret, "`^`.",
    CaretEq, "`^=`.",
    Close(Delimiter), "A close delimiter: `)`, `}`, or `]`.",
    Colon, "`:`.",
    ColonColon, "`::`.",
    Comma, "`,`.",
    Crate, "The `crate` keyword.",
    Dash, "`-`.",
    DashEq, "`-=`.",
    Default, "The `default` keyword.",
    Div, "`/`.",
    Do, "The `do` keyword.",
    Dollar, "`$`.",
    Dot, "`.`.",
    DotDot, "`..`.",
    Else, "The `else` keyword.",
    Enum, "The `enum` keyword.",
    Eq, "`=`.",
    EqEq, "`==`.",
    Extern, "The `extern` keyword.",
    False, "The `false` keyword.",
    Final, "The `final` keyword.",
    Fn, "The `fn` keyword.",
    For, "The `for` keyword.",
    Gt, "`>`.",
    GtEq, "`>=`.",
    GtGt, "`>>`.",
    GtGtEq, "`>>=`.",
    Ident(StringSource), "An identifier.",
    If, "The `if` keyword.",
    Impl, "The `impl` keyword.",
    In, "The `in` keyword.",
    Is, "The `is` keyword.",
    Label(StringSource), "A label, like `'loop`.",
    Let, "The `let` keyword.",
    LitByte(CopySource<u8>), "A byte literal.",
    LitByteStr(LitByteStrSource), "A byte string literal, including escape sequences. Like `b\"hello\\nworld\"`.",
    LitChar(CopySource<char>), "A characer literal.",
    LitNumber(NumberSource), "A number literal, like `42` or `3.14` or `0xff`.",
    LitStr(LitStrSource), "A string literal, including escape sequences. Like `\"hello\\nworld\"`.",
    LitTemplate(LitStrSource), "A template literal, including escape sequences. Like ``hello {name}``.",
    Loop, "The `loop` keyword.",
    Lt, "`<`.",
    LtEq, "`<=`.",
    LtLt, "`<<`.",
    LtLtEq, "`<<=`.",
    Macro, "The `macro` keyword.",
    Match, "The `match` keyword.",
    Mod, "The `mod` keyword.",
    Move, "The `move` keyword.",
    Not, "The `not` keyword.",
    OffsetOf, "The `offsetof` keyword.",
    Open(Delimiter), "An open delimiter: `(`, `{`, or `[`.",
    Override, "The `override` keyword.",
    Perc, "`%`.",
    PercEq, "`%=`.",
    Pipe, "`|`.",
    PipeEq, "|=`.",
    PipePipe, "`||`.",
    Plus, "`+`.",
    PlusEq, "`+=`.",
    Pound, "`#`.",
    Priv, "The `priv` keyword.",
    Proc, "The `proc` keyword.",
    Pure, "The `pure` keyword.",
    QuestionMark, "`?`.",
    Ref, "The `ref` keyword.",
    Return, "The `return` keyword.",
    Rocket, "`=>`.",
    Select, "The `select` keyword.",
    Self_, "The `self` keyword.",
    SemiColon, "`;`.",
    SizeOf, "The `sizeof` keyword.",
    SlashEq, "`/=`.",
    Star, "`*`.",
    StarEq, "`*=`.",
    Static, "The `static` keyword.",
    Struct, "The `struct` keyword.",
    Super, "The `super` keyword.",
    Tilde, "`~`.",
    True, "The `true` keyword.",
    TypeOf, "The `typeof` keyword.",
    Underscore, "`_`.",
    Unsafe, "The `unsafe` keyword.",
    Use, "The `use` keyword.",
    Virtual, "The `virtual` keyword.",
    While, "The `while` keyword.",
    Yield, "The `yield` keyword.",
    EOF, "The symbolic end of  a file.",
}

impl Kind {
    /// Try to convert an identifier into a keyword.
    pub fn from_keyword(ident: &str) -> Option<Self> {
        Some(match ident {
            "abstract" => Self::Abstract,
            "alignof" => Self::AlignOf,
            "as" => Self::As,
            "async" => Self::Async,
            "await" => Self::Await,
            "become" => Self::Become,
            "break" => Self::Break,
            "crate" => Self::Crate,
            "default" => Self::Default,
            "do" => Self::Do,
            "else" => Self::Else,
            "enum" => Self::Enum,
            "extern" => Self::Extern,
            "false" => Self::False,
            "final" => Self::Final,
            "fn" => Self::Fn,
            "for" => Self::For,
            "if" => Self::If,
            "impl" => Self::Impl,
            "in" => Self::In,
            "is" => Self::Is,
            "let" => Self::Let,
            "loop" => Self::Loop,
            "macro" => Self::Macro,
            "match" => Self::Match,
            "mod" => Self::Mod,
            "move" => Self::Move,
            "not" => Self::Not,
            "offsetof" => Self::OffsetOf,
            "override" => Self::Override,
            "priv" => Self::Priv,
            "proc" => Self::Proc,
            "pure" => Self::Pure,
            "ref" => Self::Ref,
            "return" => Self::Return,
            "select" => Self::Select,
            "self" => Self::Self_,
            "sizeof" => Self::SizeOf,
            "static" => Self::Static,
            "struct" => Self::Struct,
            "super" => Self::Super,
            "true" => Self::True,
            "typeof" => Self::TypeOf,
            "unsafe" => Self::Unsafe,
            "use" => Self::Use,
            "virtual" => Self::Virtual,
            "while" => Self::While,
            "yield" => Self::Yield,
            _ => return None,
        })
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Abstract => write!(f, "abstract")?,
            Self::AlignOf => write!(f, "alignof")?,
            Self::Amp => write!(f, "&")?,
            Self::AmpAmp => write!(f, "&&")?,
            Self::AmpEq => write!(f, "&=")?,
            Self::As => write!(f, "as")?,
            Self::Async => write!(f, "async")?,
            Self::At => write!(f, "@")?,
            Self::Arrow => write!(f, "->")?,
            Self::Await => write!(f, "await")?,
            Self::Bang => write!(f, "!")?,
            Self::BangEq => write!(f, "!=")?,
            Self::Become => write!(f, "become")?,
            Self::Break => write!(f, "break")?,
            Self::Caret => write!(f, "^")?,
            Self::CaretEq => write!(f, "^=")?,
            Self::Close(delimiter) => write!(f, "{}", delimiter.close())?,
            Self::Colon => write!(f, ":")?,
            Self::ColonColon => write!(f, "::")?,
            Self::Comma => write!(f, ",")?,
            Self::Crate => write!(f, "crate")?,
            Self::Dash => write!(f, "-")?,
            Self::DashEq => write!(f, "-=")?,
            Self::Default => write!(f, "default")?,
            Self::Div => write!(f, "/")?,
            Self::Do => write!(f, "do")?,
            Self::Dollar => write!(f, "$")?,
            Self::Dot => write!(f, ".")?,
            Self::DotDot => write!(f, "..")?,
            Self::Else => write!(f, "else")?,
            Self::Enum => write!(f, "enum")?,
            Self::Eq => write!(f, "=")?,
            Self::EqEq => write!(f, "==")?,
            Self::Extern => write!(f, "extern")?,
            Self::False => write!(f, "false")?,
            Self::Final => write!(f, "final")?,
            Self::Fn => write!(f, "fn")?,
            Self::For => write!(f, "for")?,
            Self::Gt => write!(f, ">")?,
            Self::GtEq => write!(f, ">=")?,
            Self::GtGt => write!(f, ">>")?,
            Self::GtGtEq => write!(f, ">>=")?,
            Self::Ident(..) => write!(f, "ident")?,
            Self::If => write!(f, "if")?,
            Self::Impl => write!(f, "impl")?,
            Self::In => write!(f, "in")?,
            Self::Is => write!(f, "is")?,
            Self::Label(..) => write!(f, "label")?,
            Self::Let => write!(f, "let")?,
            Self::LitByte { .. } => write!(f, "byte")?,
            Self::LitByteStr { .. } => write!(f, "byte string")?,
            Self::LitChar { .. } => write!(f, "char")?,
            Self::LitNumber { .. } => write!(f, "number")?,
            Self::LitStr { .. } => write!(f, "string")?,
            Self::LitTemplate { .. } => write!(f, "template")?,
            Self::Loop => write!(f, "loop")?,
            Self::Lt => write!(f, "<")?,
            Self::LtEq => write!(f, "<=")?,
            Self::LtLt => write!(f, "<<")?,
            Self::LtLtEq => write!(f, "<<=")?,
            Self::Macro => write!(f, "macro")?,
            Self::Match => write!(f, "match")?,
            Self::Mod => write!(f, "mod")?,
            Self::Move => write!(f, "move")?,
            Self::Not => write!(f, "not")?,
            Self::OffsetOf => write!(f, "offsetof")?,
            Self::Open(delimiter) => write!(f, "{}", delimiter.open())?,
            Self::Override => write!(f, "override")?,
            Self::Perc => write!(f, "%")?,
            Self::PercEq => write!(f, "%=")?,
            Self::Pipe => write!(f, "|")?,
            Self::PipeEq => write!(f, "|=")?,
            Self::PipePipe => write!(f, "||")?,
            Self::Plus => write!(f, "+")?,
            Self::PlusEq => write!(f, "+=")?,
            Self::Pound => write!(f, "#")?,
            Self::Priv => write!(f, "priv")?,
            Self::Proc => write!(f, "proc")?,
            Self::Pure => write!(f, "pure")?,
            Self::QuestionMark => write!(f, "?")?,
            Self::Ref => write!(f, "ref")?,
            Self::Return => write!(f, "return")?,
            Self::Rocket => write!(f, "=>")?,
            Self::Select => write!(f, "select")?,
            Self::Self_ => write!(f, "self")?,
            Self::SemiColon => write!(f, ";")?,
            Self::SizeOf => write!(f, "sizeof")?,
            Self::SlashEq => write!(f, "/=")?,
            Self::Star => write!(f, "*")?,
            Self::StarEq => write!(f, "*=")?,
            Self::Static => write!(f, "static")?,
            Self::Struct => write!(f, "struct")?,
            Self::Super => write!(f, "super")?,
            Self::Tilde => write!(f, "~")?,
            Self::True => write!(f, "true")?,
            Self::TypeOf => write!(f, "typeof")?,
            Self::Underscore => write!(f, "_")?,
            Self::Unsafe => write!(f, "unsafe")?,
            Self::Use => write!(f, "use")?,
            Self::Virtual => write!(f, "virtual")?,
            Self::While => write!(f, "while")?,
            Self::Yield => write!(f, "yield")?,
            Self::EOF => {}
        }

        Ok(())
    }
}

impl crate::IntoTokens for Kind {
    fn into_tokens(&self, context: &mut crate::MacroContext, stream: &mut crate::TokenStream) {
        stream.push(Token {
            kind: *self,
            span: context.default_span(),
        });
    }
}
