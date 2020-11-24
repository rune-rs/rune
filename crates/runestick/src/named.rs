use crate::{InstallInto, RawStr};

/// Something that is named.
pub trait Named {
    /// The name of the named thing.
    const NAME: RawStr;
}

impl Named for String {
    const NAME: RawStr = RawStr::from_str("String");
}

impl InstallInto for String {}

impl Named for i64 {
    const NAME: RawStr = RawStr::from_str("int");
}

impl InstallInto for i64 {}

impl Named for f64 {
    const NAME: RawStr = RawStr::from_str("float");
}

impl InstallInto for f64 {}

impl Named for u8 {
    const NAME: RawStr = RawStr::from_str("byte");
}

impl InstallInto for u8 {}

impl Named for char {
    const NAME: RawStr = RawStr::from_str("char");
}

impl InstallInto for char {}

impl Named for bool {
    const NAME: RawStr = RawStr::from_str("bool");
}

impl InstallInto for bool {}
