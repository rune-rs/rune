use std::fmt;

/// A span corresponding to a range in the source file being parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Span {
    /// The start of the span in bytes.
    pub start: usize,
    /// The end of the span in bytes.
    pub end: usize,
}

impl Span {
    /// Join this span with another span.
    pub fn join(self, other: Self) -> Self {
        Self {
            start: usize::min(self.start, other.start),
            end: usize::min(self.end, other.end),
        }
    }

    /// Get the point span.
    pub fn point(pos: usize) -> Self {
        Self {
            start: pos,
            end: pos,
        }
    }

    /// Narrow the span with the given amount.
    pub fn narrow(self, amount: usize) -> Self {
        Self {
            start: self.start.saturating_add(amount),
            end: self.end.saturating_sub(amount),
        }
    }

    /// Return the zero-based line and column.
    pub fn line_col(self, source: &str) -> (usize, usize) {
        let mut line = 0;
        let mut col = 0;

        let mut it = source.chars();
        let mut count = 0;

        while let Some(c) = it.next() {
            if count >= self.start {
                break;
            }

            count += c.encode_utf8(&mut [0u8; 4]).len();

            if let '\n' | '\r' = c {
                if c == '\n' {
                    line += 1;
                }

                if col > 0 {
                    col = 0;
                }

                continue;
            }

            col += 1;
        }

        (line, col)
    }
}

/// The kind of a number literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NumberLiteral {
    /// A decimal number literal, like `3.14`.
    Decimal,
    /// A hex literal, like `0xffff`.
    Hex,
    /// An octal literal, like `0o7711`.
    Octal,
    /// A binary literal, like `0b110011`.
    Binary,
}

impl fmt::Display for NumberLiteral {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Decimal => write!(fmt, "decimal"),
            Self::Hex => write!(fmt, "hex"),
            Self::Octal => write!(fmt, "octal"),
            Self::Binary => write!(fmt, "binary"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Delimiter {
    /// A parenthesis delimiter `{` and `}`.
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
    /// An `fn` token.
    Fn,
    /// A `let` token.
    Let,
    /// An `if` token.
    If,
    /// An `else` token.
    Else,
    /// An `import` token.
    Import,
    /// A `while` token.
    While,
    /// The `true` literal.
    True,
    /// The `false` literal.
    False,
    /// An identifier.
    Ident,
    /// A number literal, like `42` or `3.14` or `0xff`.
    NumberLiteral {
        /// The number literal kind.
        number: NumberLiteral,
    },
    /// A string literal, including escape sequences. Like `"hello\nworld"`.
    StringLiteral {
        /// If the string literal contains escapes.
        escaped: bool,
    },
    /// An open delimiter: `(`, `{`, or `[`.
    Open {
        /// The delimiter being opened.
        delimiter: Delimiter,
    },
    /// A close delimiter: `)`, `}`, or `]`.
    Close {
        /// The delimiter being closed.
        delimiter: Delimiter,
    },
    /// A dot `.`.
    Dot,
    /// A scope `::`.
    Scope,
    /// A comma `,`.
    Comma,
    /// A colon `:`.
    Colon,
    /// A semi-colon `;`.
    SemiColon,
    /// A plus sign `+`.
    Plus,
    /// A dash literal `-`.
    Minus,
    /// A slash literal `/`.
    Slash,
    /// A star literal `*`.
    Star,
    /// An equals sign `=`.
    Eq,
    /// Two equals sign `==`.
    EqEq,
    /// Less than comparison `<`.
    Lt,
    /// Greater than comparison `>`.
    Gt,
    /// Less than or equal comparison `<=`.
    Lte,
    /// Greater than or equal comparison `>=`.
    Gte,
}

impl fmt::Display for Kind {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Fn => write!(fmt, "fn"),
            Self::Let => write!(fmt, "let"),
            Self::If => write!(fmt, "if"),
            Self::Else => write!(fmt, "else"),
            Self::Import => write!(fmt, "import"),
            Self::While => write!(fmt, "while"),
            Self::True => write!(fmt, "true"),
            Self::False => write!(fmt, "false"),
            Self::Ident => write!(fmt, "ident"),
            Self::NumberLiteral { number } => write!(fmt, "{}", number),
            Self::StringLiteral { .. } => write!(fmt, "string"),
            Self::Open { delimiter } => write!(fmt, "{}", delimiter.open()),
            Self::Close { delimiter } => write!(fmt, "{}", delimiter.close()),
            Self::Comma => write!(fmt, ","),
            Self::Colon => write!(fmt, ":"),
            Self::Dot => write!(fmt, "."),
            Self::Scope => write!(fmt, "::"),
            Self::SemiColon => write!(fmt, ";"),
            Self::Plus => write!(fmt, "+"),
            Self::Minus => write!(fmt, "-"),
            Self::Slash => write!(fmt, "/"),
            Self::Star => write!(fmt, "*"),
            Self::Eq => write!(fmt, "="),
            Self::EqEq => write!(fmt, "=="),
            Self::Lt => write!(fmt, "<"),
            Self::Gt => write!(fmt, ">"),
            Self::Lte => write!(fmt, "<="),
            Self::Gte => write!(fmt, ">="),
        }
    }
}

/// A single token used during parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Token {
    /// The span of the token.
    pub span: Span,
    /// The kind of the token.
    pub kind: Kind,
}
