use std::fmt;

/// Result alias for the st crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error raised from a user functions.
#[derive(Debug)]
pub struct Error {
    error: anyhow::Error,
}

impl std::ops::Deref for Error {
    type Target = dyn std::error::Error + Send + Sync + 'static;

    fn deref(&self) -> &Self::Target {
        &*self.error
    }
}

impl AsRef<dyn std::error::Error + Send + Sync> for Error {
    fn as_ref(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
        &*self.error
    }
}

impl<E> From<E> for Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(error: E) -> Self {
        Self {
            error: anyhow::Error::new(error),
        }
    }
}

impl Error {
    /// A message as an error.
    pub fn msg<M>(message: M) -> Self
    where
        M: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        Self {
            error: anyhow::Error::msg(message),
        }
    }

    /// Downcast the error.
    pub fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: 'static + std::error::Error + Send + Sync,
    {
        self.error.downcast_ref()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, fmt)
    }
}
