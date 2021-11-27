use crate::ast::prelude::*;

/// A literal value
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub enum Lit {
    /// A boolean literal
    Bool(ast::LitBool),
    /// A byte literal
    Byte(ast::LitByte),
    /// A string literal
    Str(ast::LitStr),
    /// A byte string literal
    ByteStr(ast::LitByteStr),
    /// A character literal
    Char(ast::LitChar),
    /// A number literal
    Number(ast::LitNumber),
}

impl Lit {
    /// Test if this is an immediate literal in an expression.
    ///
    /// Here we only test for unambiguous literals which will not be caused by
    /// a later stage as an expression is being parsed.
    ///
    /// These include:
    /// * Object literals that start with a path (handled in [ast::Expr::parse_with_meta_path]).
    /// * Tuple literals that start with a path (handled in [ast::Expr::parse_open_paren]).
    pub(crate) fn peek_in_expr(p: &mut Peeker<'_>) -> bool {
        match p.nth(0) {
            K![true] | K![false] => true,
            K![byte] => true,
            K![number] => true,
            K![char] => true,
            K![str] => true,
            K![bytestr] => true,
            _ => false,
        }
    }
}

/// Parsing a Lit
///
/// # Examples
///
/// ```
/// use rune::{ast, testing};
///
/// testing::roundtrip::<ast::Lit>("true");
/// testing::roundtrip::<ast::Lit>("false");
/// testing::roundtrip::<ast::Lit>("'🔥'");
/// testing::roundtrip::<ast::Lit>("b'4'");
/// testing::roundtrip::<ast::Lit>("b\"bytes\"");
/// testing::roundtrip::<ast::Lit>("1.2");
/// testing::roundtrip::<ast::Lit>("42");
/// testing::roundtrip::<ast::Lit>("\"mary had a little lamb\"");
/// ```
impl Parse for Lit {
    fn parse(p: &mut Parser<'_>) -> Result<Self, ParseError> {
        match p.nth(0)? {
            K![true] | K![false] => return Ok(Lit::Bool(p.parse()?)),
            K![byte(_)] => return Ok(Lit::Byte(p.parse()?)),
            K![number(_)] => return Ok(Lit::Number(p.parse()?)),
            K![char(_)] => return Ok(Lit::Char(p.parse()?)),
            K![str(_)] => return Ok(Lit::Str(p.parse()?)),
            K![bytestr(_)] => return Ok(Lit::ByteStr(p.parse()?)),
            _ => (),
        }

        Err(ParseError::expected(p.next()?, Expectation::Literal))
    }
}
