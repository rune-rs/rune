use core::fmt;

/// Something that describes an expectation or actuality.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Expectation {
    /// A static description.
    Description(&'static str),
    /// A keyword like `await`.
    Keyword(&'static str),
    /// A delimiter.
    Delimiter(&'static str),
    /// A punctuation which can be a sequence of characters, like `!=`.
    Punctuation(&'static str),
    /// Expected a specific kind of syntax node.
    Syntax,
    /// An open delimiter.
    OpenDelimiter,
    /// A bolean.
    Boolean,
    /// A literal.
    Literal,
    /// An expression.
    Expression,
    /// A shebang.
    Shebang,
    /// A comment.
    Comment,
}

impl fmt::Display for Expectation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expectation::Description(s) => s.fmt(f),
            Expectation::Keyword(k) => k.fmt(f),
            Expectation::Delimiter(d) => write!(f, "`{}`", d),
            Expectation::Punctuation(p) => write!(f, "`{}`", p),
            Expectation::OpenDelimiter => write!(f, "`(`, `[`, or `{{`"),
            Expectation::Boolean => write!(f, "true or false"),
            Expectation::Literal => write!(f, r#"literal like `"a string"` or 42"#),
            Expectation::Expression => write!(f, "expression"),
            Expectation::Shebang => write!(f, "shebang"),
            Expectation::Comment => write!(f, "comment"),
            Expectation::Syntax => write!(f, "syntax"),
        }
    }
}

/// Helper trait to get description.
pub(crate) trait IntoExpectation {
    /// Get the description for the thing.
    fn into_expectation(self) -> Expectation;
}

impl IntoExpectation for Expectation {
    fn into_expectation(self) -> Expectation {
        self
    }
}

impl IntoExpectation for &'static str {
    fn into_expectation(self) -> Expectation {
        Expectation::Description(self)
    }
}
