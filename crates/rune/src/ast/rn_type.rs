use crate::ast::prelude::*;

#[test]
#[cfg(not(miri))]
fn ast_parse() {
    rt::<ast::Type>("Bar");
    rt::<ast::Type>("do::re::mi::fa::sol::la::si::Do");
    rt::<ast::Type>("Self");
}

/// A type, used for static typing.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub enum Type {
	/// If the type is an identifier or a path.
	Path(ast::Path),
	/// If the type is "Self".
	SelfType(T![Self]),
	/// If the type should return nothing (a.k.a the "never" type in Rust).
	Bang(T![!]),
}

impl Parse for Type {
    fn parse(p: &mut Parser<'_>) -> Result<Self> {
        let segment = match p.nth(0)? {
            K![Self] => Self::SelfType(p.parse()?),
            K![ident] => Self::Path(p.parse()?),
            K![!] => Self::Bang(p.parse()?),
            _ => {
                return Err(compile::Error::expected(p.tok_at(0)?, "type"));
            }
        };

        Ok(segment)
    }
}