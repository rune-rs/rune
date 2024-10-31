use musli::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate as rune;
use crate::compile::Named;
use crate::module::InstallWith;
use crate::runtime::RuntimeError;
use crate::{item, FromValue, Hash, Item, Value};

/// A value representing a type in the virtual machine.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Decode, Encode,
)]
#[repr(transparent)]
#[serde(transparent)]
#[musli(transparent)]
pub struct Type(Hash);

impl Type {
    /// Construct a new type.
    pub(crate) fn new(hash: Hash) -> Self {
        Self(hash)
    }

    /// Coerce into inner type hash.
    #[inline]
    pub fn into_hash(self) -> Hash {
        self.0
    }
}

impl InstallWith for Type {}

impl FromValue for Type {
    #[inline]
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        value.as_type()
    }
}

impl Named for Type {
    const ITEM: &'static Item = item!(::std::any::Type);
}
