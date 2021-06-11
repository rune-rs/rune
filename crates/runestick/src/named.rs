use crate::{InstallWith, RawStr};

/// Something that is named.
pub trait Named {
    /// The generic name of the named thing.
    const BASE_NAME: RawStr;

    /// The exact type name
    fn full_name() -> String {
        (*Self::BASE_NAME).to_owned()
    }
}

impl Named for String {
    const BASE_NAME: RawStr = RawStr::from_str("String");
}

impl InstallWith for String {}

impl Named for i64 {
    const BASE_NAME: RawStr = RawStr::from_str("int");
}

impl InstallWith for i64 {}

impl Named for f64 {
    const BASE_NAME: RawStr = RawStr::from_str("float");
}

impl InstallWith for f64 {}

impl Named for u8 {
    const BASE_NAME: RawStr = RawStr::from_str("byte");
}

impl InstallWith for u8 {}

impl Named for char {
    const BASE_NAME: RawStr = RawStr::from_str("char");
}

impl InstallWith for char {}

impl Named for bool {
    const BASE_NAME: RawStr = RawStr::from_str("bool");
}

impl InstallWith for bool {}
