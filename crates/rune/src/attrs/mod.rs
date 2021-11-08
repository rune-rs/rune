mod attributes;
use crate::ast;
use crate::{Parse, ParseError, Resolve as _, Storage};
use runestick::Source;

pub(crate) use self::attributes::Attributes;

pub(crate) trait Attribute {
    const PATH: &'static str;
}

#[derive(Default)]
pub(crate) struct BuiltInArgs {
    pub(crate) literal: bool,
}

#[derive(Parse)]
pub(crate) struct BuiltIn {
    /// Arguments to this built-in.
    pub args: Option<ast::Parenthesized<ast::Ident, T![,]>>,
}

impl BuiltIn {
    /// Parse built-in arguments.
    pub(crate) fn args(
        &self,
        storage: &Storage,
        source: &Source,
    ) -> Result<BuiltInArgs, ParseError> {
        let mut out = BuiltInArgs::default();

        if let Some(args) = &self.args {
            for (ident, _) in args {
                match ident.resolve(storage, source)?.as_ref() {
                    "literal" => {
                        out.literal = true;
                    }
                    _ => {
                        return Err(ParseError::msg(ident, "unsupported attribute"));
                    }
                }
            }
        }

        Ok(out)
    }
}

impl Attribute for BuiltIn {
    /// Must match the specified name.
    const PATH: &'static str = "builtin";
}

/// NB: at this point we don't support attributes beyond the empty `#[test]`.
#[derive(Parse)]
pub(crate) struct Test {}

impl Attribute for Test {
    /// Must match the specified name.
    const PATH: &'static str = "test";
}

/// NB: at this point we don't support attributes beyond the empty `#[bench]`.
#[derive(Parse)]
pub(crate) struct Bench {}

impl Attribute for Bench {
    /// Must match the specified name.
    const PATH: &'static str = "bench";
}
