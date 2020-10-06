use crate::ast;
use crate::macros;
use crate::shared;
use std::fmt;

/// This file has been generated from `assets\tokens.yaml`
/// DO NOT modify by hand!

/// The kind of the token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    /// A close delimiter: `)`, `}`, or `]`.
    Close(ast::Delimiter),
    /// An open delimiter: `(`, `{`, or `[`.
    Open(ast::Delimiter),
    /// An identifier.
    Ident(ast::StringSource),
    /// A label, like `'loop`.
    Label(ast::StringSource),
    /// A byte literal.
    LitByte(ast::CopySource<u8>),
    /// A byte string literal, including escape sequences. Like `b"hello\nworld"`.
    LitByteStr(ast::LitStrSource),
    /// A characer literal.
    LitChar(ast::CopySource<char>),
    /// A number literal, like `42` or `3.14` or `0xff`.
    LitNumber(ast::NumberSource),
    /// A string literal, including escape sequences. Like `"hello\nworld"`.
    LitStr(ast::LitStrSource),
    /// The `abstract` keyword.
    Abstract,
    /// The `alignof` keyword.
    AlignOf,
    /// `&`.
    Amp,
    /// `&&`.
    AmpAmp,
    /// `&=`.
    AmpEq,
    /// `->`.
    Arrow,
    /// The `as` keyword.
    As,
    /// The `async` keyword.
    Async,
    /// `@`.
    At,
    /// The `await` keyword.
    Await,
    /// `!`.
    Bang,
    /// `!=`.
    BangEq,
    /// The `become` keyword.
    Become,
    /// The `break` keyword.
    Break,
    /// `^`.
    Caret,
    /// `^=`.
    CaretEq,
    /// `:`.
    Colon,
    /// `::`.
    ColonColon,
    /// `,`.
    Comma,
    /// The `const` keyword.
    Const,
    /// The `crate` keyword.
    Crate,
    /// `-`.
    Dash,
    /// `-=`.
    DashEq,
    /// The `default` keyword.
    Default,
    /// `/`.
    Div,
    /// The `do` keyword.
    Do,
    /// `$`.
    Dollar,
    /// `.`.
    Dot,
    /// `..`.
    DotDot,
    /// The `else` keyword.
    Else,
    /// The `enum` keyword.
    Enum,
    /// `=`.
    Eq,
    /// `==`.
    EqEq,
    /// The `extern` keyword.
    Extern,
    /// The `false` keyword.
    False,
    /// The `final` keyword.
    Final,
    /// The `fn` keyword.
    Fn,
    /// The `for` keyword.
    For,
    /// `>`.
    Gt,
    /// `>=`.
    GtEq,
    /// `>>`.
    GtGt,
    /// `>>=`.
    GtGtEq,
    /// The `if` keyword.
    If,
    /// The `impl` keyword.
    Impl,
    /// The `in` keyword.
    In,
    /// The `is` keyword.
    Is,
    /// The `let` keyword.
    Let,
    /// The `loop` keyword.
    Loop,
    /// `<`.
    Lt,
    /// `<=`.
    LtEq,
    /// `<<`.
    LtLt,
    /// `<<=`.
    LtLtEq,
    /// The `macro` keyword.
    Macro,
    /// The `match` keyword.
    Match,
    /// The `mod` keyword.
    Mod,
    /// The `move` keyword.
    Move,
    /// The `not` keyword.
    Not,
    /// The `offsetof` keyword.
    OffsetOf,
    /// The `override` keyword.
    Override,
    /// `%`.
    Perc,
    /// `%=`.
    PercEq,
    /// `|`.
    Pipe,
    /// |=`.
    PipeEq,
    /// `||`.
    PipePipe,
    /// `+`.
    Plus,
    /// `+=`.
    PlusEq,
    /// `#`.
    Pound,
    /// The `priv` keyword.
    Priv,
    /// The `proc` keyword.
    Proc,
    /// The `pub` keyword.
    Pub,
    /// The `pure` keyword.
    Pure,
    /// `?`.
    QuestionMark,
    /// The `ref` keyword.
    Ref,
    /// The `return` keyword.
    Return,
    /// `=>`.
    Rocket,
    /// The `select` keyword.
    Select,
    /// The `Self` keyword.
    SelfType,
    /// The `self` keyword.
    SelfValue,
    /// `;`.
    SemiColon,
    /// The `sizeof` keyword.
    SizeOf,
    /// `/=`.
    SlashEq,
    /// `*`.
    Star,
    /// `*=`.
    StarEq,
    /// The `static` keyword.
    Static,
    /// The `struct` keyword.
    Struct,
    /// The `super` keyword.
    Super,
    /// The `template` keyword.
    Template,
    /// `~`.
    Tilde,
    /// The `true` keyword.
    True,
    /// The `typeof` keyword.
    TypeOf,
    /// `_`.
    Underscore,
    /// The `unsafe` keyword.
    Unsafe,
    /// The `use` keyword.
    Use,
    /// The `virtual` keyword.
    Virtual,
    /// The `while` keyword.
    While,
    /// The `yield` keyword.
    Yield,
}

impl From<ast::Token> for Kind {
    fn from(token: ast::Token) -> Self {
        token.kind
    }
}

impl Kind {
    /// Try to convert an identifier into a keyword.
    pub fn from_keyword(ident: &str) -> Option<Self> {
        match ident {
            "abstract" => Some(Self::Abstract),
            "alignof" => Some(Self::AlignOf),
            "as" => Some(Self::As),
            "async" => Some(Self::Async),
            "await" => Some(Self::Await),
            "become" => Some(Self::Become),
            "break" => Some(Self::Break),
            "const" => Some(Self::Const),
            "crate" => Some(Self::Crate),
            "default" => Some(Self::Default),
            "do" => Some(Self::Do),
            "else" => Some(Self::Else),
            "enum" => Some(Self::Enum),
            "extern" => Some(Self::Extern),
            "false" => Some(Self::False),
            "final" => Some(Self::Final),
            "fn" => Some(Self::Fn),
            "for" => Some(Self::For),
            "if" => Some(Self::If),
            "impl" => Some(Self::Impl),
            "in" => Some(Self::In),
            "is" => Some(Self::Is),
            "let" => Some(Self::Let),
            "loop" => Some(Self::Loop),
            "macro" => Some(Self::Macro),
            "match" => Some(Self::Match),
            "mod" => Some(Self::Mod),
            "move" => Some(Self::Move),
            "not" => Some(Self::Not),
            "offsetof" => Some(Self::OffsetOf),
            "override" => Some(Self::Override),
            "priv" => Some(Self::Priv),
            "proc" => Some(Self::Proc),
            "pub" => Some(Self::Pub),
            "pure" => Some(Self::Pure),
            "ref" => Some(Self::Ref),
            "return" => Some(Self::Return),
            "select" => Some(Self::Select),
            "Self" => Some(Self::SelfType),
            "self" => Some(Self::SelfValue),
            "sizeof" => Some(Self::SizeOf),
            "static" => Some(Self::Static),
            "struct" => Some(Self::Struct),
            "super" => Some(Self::Super),
            "template" => Some(Self::Template),
            "true" => Some(Self::True),
            "typeof" => Some(Self::TypeOf),
            "unsafe" => Some(Self::Unsafe),
            "use" => Some(Self::Use),
            "virtual" => Some(Self::Virtual),
            "while" => Some(Self::While),
            "yield" => Some(Self::Yield),
            _ => None,
        }
    }

    /// Get the kind as a descriptive string.
    fn as_str(self) -> &'static str {
        match self {
            Self::Close(delimiter) => delimiter.close(),
            Self::Open(delimiter) => delimiter.open(),
            Self::Ident(..) => "ident",
            Self::Label(..) => "label",
            Self::LitByte { .. } => "byte",
            Self::LitByteStr { .. } => "byte string",
            Self::LitChar { .. } => "char",
            Self::LitNumber { .. } => "number",
            Self::LitStr { .. } => "string",
            Self::Abstract => "abstract",
            Self::AlignOf => "alignof",
            Self::Amp => "&",
            Self::AmpAmp => "&&",
            Self::AmpEq => "&=",
            Self::Arrow => "->",
            Self::As => "as",
            Self::Async => "async",
            Self::At => "@",
            Self::Await => "await",
            Self::Bang => "!",
            Self::BangEq => "!=",
            Self::Become => "become",
            Self::Break => "break",
            Self::Caret => "^",
            Self::CaretEq => "^=",
            Self::Colon => ":",
            Self::ColonColon => "::",
            Self::Comma => ",",
            Self::Const => "const",
            Self::Crate => "crate",
            Self::Dash => "-",
            Self::DashEq => "-=",
            Self::Default => "default",
            Self::Div => "/",
            Self::Do => "do",
            Self::Dollar => "$",
            Self::Dot => ".",
            Self::DotDot => "..",
            Self::Else => "else",
            Self::Enum => "enum",
            Self::Eq => "=",
            Self::EqEq => "==",
            Self::Extern => "extern",
            Self::False => "false",
            Self::Final => "final",
            Self::Fn => "fn",
            Self::For => "for",
            Self::Gt => ">",
            Self::GtEq => ">=",
            Self::GtGt => ">>",
            Self::GtGtEq => ">>=",
            Self::If => "if",
            Self::Impl => "impl",
            Self::In => "in",
            Self::Is => "is",
            Self::Let => "let",
            Self::Loop => "loop",
            Self::Lt => "<",
            Self::LtEq => "<=",
            Self::LtLt => "<<",
            Self::LtLtEq => "<<=",
            Self::Macro => "macro",
            Self::Match => "match",
            Self::Mod => "mod",
            Self::Move => "move",
            Self::Not => "not",
            Self::OffsetOf => "offsetof",
            Self::Override => "override",
            Self::Perc => "%",
            Self::PercEq => "%=",
            Self::Pipe => "|",
            Self::PipeEq => "|=",
            Self::PipePipe => "||",
            Self::Plus => "+",
            Self::PlusEq => "+=",
            Self::Pound => "#",
            Self::Priv => "priv",
            Self::Proc => "proc",
            Self::Pub => "pub",
            Self::Pure => "pure",
            Self::QuestionMark => "?",
            Self::Ref => "ref",
            Self::Return => "return",
            Self::Rocket => "=>",
            Self::Select => "select",
            Self::SelfType => "Self",
            Self::SelfValue => "self",
            Self::SemiColon => ";",
            Self::SizeOf => "sizeof",
            Self::SlashEq => "/=",
            Self::Star => "*",
            Self::StarEq => "*=",
            Self::Static => "static",
            Self::Struct => "struct",
            Self::Super => "super",
            Self::Template => "template",
            Self::Tilde => "~",
            Self::True => "true",
            Self::TypeOf => "typeof",
            Self::Underscore => "_",
            Self::Unsafe => "unsafe",
            Self::Use => "use",
            Self::Virtual => "virtual",
            Self::While => "while",
            Self::Yield => "yield",
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl macros::ToTokens for Kind {
    fn to_tokens(&self, context: &macros::MacroContext, stream: &mut macros::TokenStream) {
        stream.push(ast::Token {
            kind: *self,
            span: context.span(),
        });
    }
}

impl shared::Description for Kind {
    fn description(self) -> &'static str {
        self.as_str()
    }
}
