use crate::ast::prelude::*;

#[test]
#[cfg(not(miri))]
fn ast_parse() {
    rt::<ast::Type>("Bar");
    rt::<ast::Type>("one::two::three::four::Five");
    rt::<ast::Type>("Self");
    rt::<ast::Type>("(one::One, two::Two)");
    rt::<ast::Type>("(one::One, (two::Two, three::Three))");
}

/// A type, used for static typing.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub enum Type {
    /// If the type is an identifier or a path.
    Path(ast::Path),
    /// If the type should return nothing (a.k.a the "never" type in Rust).
    Bang(T![!]),
    /// If the type is a tuple.
    Tuple(ast::Parenthesized<Box<Type>, T![,]>),
}

impl Parse for Type {
    fn parse(p: &mut Parser<'_>) -> Result<Self> {
        let segment = match p.nth(0)? {
            K![!] => Self::Bang(p.parse()?),
            K!['('] => Self::Tuple(p.parse()?),
            _ => Self::Path(p.parse()?),
        };

        Ok(segment)
    }
}
