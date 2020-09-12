use crate::{RawStr, Value};

/// Something that is named.
pub trait Named {
    /// The name of the named thing.
    const NAME: RawStr;
}

impl Named for String {
    const NAME: RawStr = RawStr::from_str("String");
}

impl Named for Vec<Value> {
    const NAME: RawStr = RawStr::from_str("Vec");
}

impl Named for i64 {
    const NAME: RawStr = RawStr::from_str("int");
}

impl Named for f64 {
    const NAME: RawStr = RawStr::from_str("float");
}

impl Named for u8 {
    const NAME: RawStr = RawStr::from_str("byte");
}

impl Named for char {
    const NAME: RawStr = RawStr::from_str("char");
}

impl Named for bool {
    const NAME: RawStr = RawStr::from_str("bool");
}
