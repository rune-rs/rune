use crate::ast::prelude::*;

#[test]
#[cfg(not(miri))]
fn ast_parse() {
    rt::<ast::FnArg>("self");
    rt::<ast::FnArg>("_");
    rt::<ast::FnArg>("abc");
}

#[test]
#[cfg(not(miri))]
fn ast_parse_typed() {
    rt::<ast::FnArg>("a: i64");
    rt::<ast::FnArg>("a: foo::Bar");
    rt::<ast::FnArg>("_: i64");
}

/// A single argument in a closure.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub enum FnArg {
    /// The `self` parameter.
    SelfValue(T![self]),
    /// Function argument is a pattern binding.
    Pat(ast::Pat),
    /// Function argument with type annotation (gradual typing).
    Typed(Box<FnArgTyped>),
}

/// A function argument with an optional type annotation.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct FnArgTyped {
    /// The pattern for this argument (typically just an identifier).
    pub pat: ast::Pat,
    /// The colon separator.
    pub colon: T![:],
    /// The type annotation.
    pub ty: ast::Type,
}

impl FnArg {
    /// Get the type annotation if present (gradual typing feature).
    pub fn ty(&self) -> Option<&ast::Type> {
        match self {
            FnArg::SelfValue(_) => None,
            FnArg::Pat(_) => None,
            FnArg::Typed(typed) => Some(&typed.ty),
        }
    }

    /// Get the pattern for this argument.
    ///
    /// Returns `None` for `self` arguments, and `Some` for pattern-based
    /// or typed arguments.
    pub fn pat(&self) -> Option<&ast::Pat> {
        match self {
            FnArg::SelfValue(_) => None,
            FnArg::Pat(pat) => Some(pat),
            FnArg::Typed(typed) => Some(&typed.pat),
        }
    }
}

impl Parse for FnArg {
    fn parse(p: &mut Parser<'_>) -> Result<Self> {
        Ok(match p.nth(0)? {
            K![self] => Self::SelfValue(p.parse()?),
            _ => {
                {
                    // Check if this is a typed argument: `name: Type` or `_: Type`
                    // We need to look ahead to distinguish from object patterns
                    // Pattern: identifier/underscore followed by `:` and then a type
                    match (p.nth(0)?, p.nth(1)?) {
                        (K![ident], K![:]) => {
                            // Parse identifier as a path pattern
                            let ident = p.parse::<ast::Ident>()?;
                            let path = ast::Path {
                                global: None,
                                first: ast::PathSegment::Ident(ident),
                                rest: Vec::new(),
                                trailing: None,
                                id: Default::default(),
                            };
                            let pat = ast::Pat::Path(ast::PatPath {
                                attributes: Vec::new(),
                                path,
                            });
                            let colon = p.parse::<T![:]>()?;
                            let ty = p.parse::<ast::Type>()?;
                            return Ok(Self::Typed(Box::try_new(FnArgTyped { pat, colon, ty })?));
                        }
                        (K![_], K![:]) => {
                            // Parse underscore as ignore pattern
                            let underscore = p.parse::<T![_]>()?;
                            let pat = ast::Pat::Ignore(ast::PatIgnore {
                                attributes: Vec::new(),
                                underscore,
                            });
                            let colon = p.parse::<T![:]>()?;
                            let ty = p.parse::<ast::Type>()?;
                            return Ok(Self::Typed(Box::try_new(FnArgTyped { pat, colon, ty })?));
                        }
                        _ => {}
                    }
                }
                Self::Pat(p.parse()?)
            }
        })
    }
}
