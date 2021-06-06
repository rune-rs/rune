use crate::{InstallWith, RawStr};

/// Something that is named.
pub trait Named {
    /// The generic name of the named thing.
    const NAME: &'static str;

    /// Get the generic name as a RawStr
    fn raw() -> RawStr {
        RawStr::from_str(Self::NAME)
    }

    /// The exact type name
    fn exact() -> String {
        Self::NAME.to_owned()
    }
}

impl Named for String {
    const NAME: &'static str = "String";
}

impl InstallWith for String {}

impl Named for i64 {
    const NAME: &'static str = "int";
}

impl InstallWith for i64 {}

impl Named for f64 {
    const NAME: &'static str = "float";
}

impl InstallWith for f64 {}

impl Named for u8 {
    const NAME: &'static str = "byte";
}

impl InstallWith for u8 {}

impl Named for char {
    const NAME: &'static str = "char";
}

impl InstallWith for char {}

impl Named for bool {
    const NAME: &'static str = "bool";
}

impl InstallWith for bool {}
