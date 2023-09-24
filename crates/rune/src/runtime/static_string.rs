use core::cmp;
use core::fmt;
use core::hash;
use core::ops;

use serde::{Deserialize, Serialize};

use crate::alloc::prelude::*;
use crate::alloc::{self, String};
use crate::hash::{Hash, IntoHash};

/// Struct representing a static string.
#[derive(Serialize, Deserialize)]
pub struct StaticString {
    inner: String,
    hash: Hash,
}

impl cmp::PartialEq for StaticString {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash && self.inner == other.inner
    }
}

impl cmp::Eq for StaticString {}

impl cmp::PartialOrd for StaticString {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl cmp::Ord for StaticString {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl hash::Hash for StaticString {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.hash.hash(state)
    }
}

impl StaticString {
    /// Construct a new static string.
    pub fn new<S>(s: S) -> alloc::Result<Self>
    where
        S: AsRef<str>,
    {
        let inner = s.as_ref().try_to_owned()?;
        let hash = s.as_ref().into_hash();
        Ok(Self { inner, hash })
    }

    /// Get the hash of the string.
    #[inline]
    pub fn hash(&self) -> Hash {
        self.hash
    }
}

impl AsRef<String> for StaticString {
    #[inline]
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
        let hash = inner.as_str().into_hash();
        Self { inner, hash }
    }
}
