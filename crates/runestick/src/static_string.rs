use std::fmt;
use std::ops;
use std::sync::Arc;

/// Struct representing a static string.
#[derive(Clone)]
#[repr(transparent)]
pub struct StaticString {
    inner: Arc<String>,
}

impl StaticString {
    /// Convert into inner.
    pub fn into_inner(self) -> Arc<String> {
        self.inner
    }
}

impl AsRef<String> for StaticString {
    fn as_ref(&self) -> &String {
        &*self.inner
    }
}

impl fmt::Debug for StaticString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", &*self.inner)?;
        Ok(())
    }
}

impl ops::Deref for StaticString {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl From<Arc<String>> for StaticString {
    fn from(inner: Arc<String>) -> Self {
        Self { inner }
    }
}
