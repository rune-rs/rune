mod attributes;
use crate::Parse;

pub(crate) use self::attributes::Attributes;

pub(crate) trait Attribute {
    const PATH: &'static str;
}

#[derive(Parse)]
pub(crate) struct BuiltIn {}

impl Attribute for BuiltIn {
    /// Must match the specified name.
    const PATH: &'static str = "builtin";
}
