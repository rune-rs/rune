/// Advance a parser by a given amount.
pub(crate) trait Advance {
    /// Error produced when advancing.
    type Error;

    /// Advance the parser by `n` tokens.
    fn advance(&mut self, n: usize) -> Result<(), Self::Error>;
}
