use crate::ast::{Path, SemiColon, Use};
use crate::error::ParseError;
use crate::parser::Parser;
use crate::traits::Parse;

/// An imported declaration.
#[derive(Debug, Clone)]
pub struct DeclUse {
    /// The use token.
    pub use_: Use,
    /// The name of the imported module.
    pub path: Path,
    /// Trailing semi-colon.
    pub semi_colon: SemiColon,
}

/// Parsing an use declaration.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// # fn main() -> anyhow::Result<()> {
/// parse_all::<ast::DeclUse>("use foo;")?;
/// parse_all::<ast::DeclUse>("use foo::bar;")?;
/// parse_all::<ast::DeclUse>("use foo::bar::baz;")?;
/// # Ok(())
/// # }
/// ```
impl Parse for DeclUse {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(Self {
            use_: parser.parse()?,
            path: parser.parse()?,
            semi_colon: parser.parse()?,
        })
    }
}
