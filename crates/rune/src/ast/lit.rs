use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::Lit>("true");
    rt::<ast::Lit>("false");
    rt::<ast::Lit>("'🔥'");
    rt::<ast::Lit>("b'4'");
    rt::<ast::Lit>("b\"bytes\"");
    rt::<ast::Lit>("1.2");
    rt::<ast::Lit>("42");
    rt::<ast::Lit>("\"mary had a little lamb\"");
}

/// A literal value,
///
/// These are made available by parsing Rune. Custom literals for macros can be
/// constructed through [MacroContext::lit][crate::macros::MacroContext::lit].
///
/// # Examples
///
/// Constructing a literal value:
///
/// ```
/// use rune::ast;
/// use rune::macros;
///
/// macros::test(|cx| {
///     let lit = cx.lit("hello world")?;
///     assert!(matches!(lit, ast::Lit::Str(..)));
///     Ok(())
/// })?;
/// # Ok::<_, rune::support::Error>(())
/// ```
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, ToTokens, Spanned)]
#[try_clone(copy)]
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

impl Parse for Lit {
    fn parse(p: &mut Parser<'_>) -> Result<Self> {
        match p.nth(0)? {
            K![true] | K![false] => return Ok(Lit::Bool(p.parse()?)),
            K![byte(_)] => return Ok(Lit::Byte(p.parse()?)),
            K![number(_)] => return Ok(Lit::Number(p.parse()?)),
            K![char(_)] => return Ok(Lit::Char(p.parse()?)),
            K![str(_)] => return Ok(Lit::Str(p.parse()?)),
            K![bytestr(_)] => return Ok(Lit::ByteStr(p.parse()?)),
            _ => (),
        }

        Err(compile::Error::expected(p.next()?, Expectation::Literal))
    }
}
