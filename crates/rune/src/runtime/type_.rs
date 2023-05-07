use serde::{Deserialize, Serialize};

use crate::compile::Named;
use crate::module::InstallWith;
use crate::runtime::{RawStr, TypeInfo, TypeOf, VmResult};
use crate::{FromValue, Hash, Value};

/// A value representing a type in the virtual machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct Type(Hash);

impl Type {
    /// Construct a new type.
    pub(crate) fn new(hash: Hash) -> Self {
        Self(hash)
    }

    /// Coerce into inner hash.
    #[inline]
    pub(crate) fn into_hash(self) -> Hash {
        self.0
    }
}

impl InstallWith for Type {}

impl Named for Type {
    const BASE_NAME: RawStr = RawStr::from_str("Type");
}

impl FromValue for Type {
    #[inline]
    fn from_value(value: Value) -> VmResult<Self> {
        VmResult::Ok(vm_try!(value.into_type()))
    }
}

impl TypeOf for Type {
    #[inline]
    fn type_hash() -> Hash {
        crate::runtime::TYPE.hash
    }

    #[inline]
    fn type_info() -> TypeInfo {
        TypeInfo::StaticType(crate::runtime::TYPE)
    }
}
