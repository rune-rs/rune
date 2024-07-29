use crate::ast::Token;

/// Advance a parser by a given amount.
pub(crate) trait Advance {
    /// Error produced when advancing.
    type Error;

    /// Advance the parser by `n` tokens.
    fn advance(&mut self, n: usize) -> Result<(), Self::Error>;
}

/// Helper when peeking.
pub(crate) trait Peekable {
    /// Error produced when peeking.
    type Error;

    /// Peek at the nth token.
    fn nth(&mut self, n: usize) -> Result<Token, Self::Error>;
}
