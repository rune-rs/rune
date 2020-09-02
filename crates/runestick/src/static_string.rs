use crate::Hash;
use std::fmt;
use std::ops;

/// Struct representing a static string.
#[derive(Clone)]
pub struct StaticString {
    inner: String,
    hash: Hash,
}

impl StaticString {
    /// Construct a new static string.
    pub fn new<S>(s: S) -> Self
    where
        S: AsRef<str>,
    {
        let inner = s.as_ref().to_owned();
        let hash = Hash::of(&inner);

        Self { inner, hash }
    }

    /// Get the hash of the string.
    pub fn hash(&self) -> Hash {
        self.hash
    }
}

impl AsRef<String> for StaticString {
    fn as_ref(&self) -> &String {
        &self.inner
    }
}

impl fmt::Debug for StaticString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", &self.inner)?;
        Ok(())
    }
}

impl ops::Deref for StaticString {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<String> for StaticString {
    fn from(inner: String) -> Self {
        let hash = Hash::of(inner.as_str());
        Self { inner, hash }
    }
}
