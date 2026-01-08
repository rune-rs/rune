use core::cmp;
use core::fmt;
use core::hash;
use core::ops;

#[cfg(feature = "musli")]
use musli_core::{Decode, Encode};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::alloc::prelude::*;
use crate::alloc::{self, String};
use crate::hash::{Hash, IntoHash};

/// Struct representing a static string.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "musli", derive(Decode, Encode), musli(crate = musli_core))]
pub struct StaticString {
    inner: String,
    hash: Hash,
}

impl cmp::PartialEq for StaticString {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash && self.inner == other.inner
    }
}

impl cmp::Eq for StaticString {}

impl cmp::PartialOrd for StaticString {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl cmp::Ord for StaticString {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl hash::Hash for StaticString {
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.hash.hash(state)
    }
}

impl StaticString {
    /// Construct a new static string.
    #[inline]
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

impl fmt::Display for StaticString {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl fmt::Debug for StaticString {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", &self.inner)
    }
}

impl ops::Deref for StaticString {
    type Target = String;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<String> for StaticString {
    #[inline]
    fn from(inner: String) -> Self {
        let hash = inner.as_str().into_hash();
        Self { inner, hash }
    }
}
