use crate::{InstallWith, RawStr};

/// Something that is named.
pub trait Named {
    /// The generic name of the named thing.
    const NAME: RawStr;

    /// The exact type name
    fn exact() -> String {
        (*Self::NAME).to_owned()
    }
}

impl Named for String {
    const NAME: RawStr = RawStr::from_str("String");
}

impl InstallWith for String {}

impl Named for i64 {
    const NAME: RawStr = RawStr::from_str("int");
}

impl InstallWith for i64 {}

impl Named for f64 {
    const NAME: RawStr = RawStr::from_str("float");
}

impl InstallWith for f64 {}

impl Named for u8 {
    const NAME: RawStr = RawStr::from_str("byte");
}

impl InstallWith for u8 {}

impl Named for char {
    const NAME: RawStr = RawStr::from_str("char");
}

impl InstallWith for char {}

impl Named for bool {
    const NAME: RawStr = RawStr::from_str("bool");
}

impl InstallWith for bool {}
