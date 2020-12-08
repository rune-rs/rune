use std::fmt;

/// Format a comma-separated sequence.
pub(crate) fn commas<I>(it: I) -> impl fmt::Display
where
    I: Copy + IntoIterator,
    I::Item: fmt::Display,
{
    Seq(it, ',')
}

/// Helper to format a character-separated sequence.
struct Seq<I>(I, char);

impl<I> fmt::Display for Seq<I>
where
    I: Copy + IntoIterator,
    I::Item: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(it, c) = self;
        let mut it = it.into_iter().peekable();

        while let Some(item) = it.next() {
            write!(f, "{}", item)?;

            if it.peek().is_some() {
                write!(f, "{} ", c)?;
            }
        }

        Ok(())
    }
}
