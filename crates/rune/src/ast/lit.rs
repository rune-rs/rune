use crate::ast;
use crate::{Parse, ParseError, Parser, Peeker, Spanned, ToTokens};

/// A literal value
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub enum Lit {
    /// A unit literal
    Unit(ast::LitUnit),
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
    /// An object literal
    Object(ast::LitObject),
    /// A tuple literal
    Tuple(ast::LitTuple),
    /// A vec literal
    Vec(ast::LitVec),
}

impl Lit {
    /// Construct a new literal.
    ///
    /// # Panics
    ///
    /// This will panic if it's called outside of a macro context.
    pub fn new<T>(lit: T) -> Self
    where
        T: crate::macros::IntoLit,
    {
        crate::macros::current_context(|ctx| ctx.lit(lit))
    }

    /// Test if this is an immediate literal in an expression.
    ///
    /// Here we only test for unambiguous literals which will not be caused by
    /// a later stage as an expression is being parsed.
    ///
    /// These include:
    /// * Object literals that start with a path (handled in [ast::Expr::parse_ident_start]).
    /// * Tuple literals that start with a path (handled in [ast::Expr::parse_open_paren]).
    pub(crate) fn peek_in_expr(p: &mut Peeker<'_>) -> bool {
        match p.nth(0) {
            K![true] | K![false] => true,
            K![byte] => true,
            K![number] => true,
            K![char] => true,
            K![str] => true,
            K![bytestr] => true,
            K!['['] => true,
            K![#] => matches!(p.nth(1), K!['{']),
            _ => false,
        }
    }
}

/// Parsing a Lit
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::Lit>("()");
/// testing::roundtrip::<ast::Lit>("true");
/// testing::roundtrip::<ast::Lit>("false");
/// testing::roundtrip::<ast::Lit>("'🔥'");
/// testing::roundtrip::<ast::Lit>("b'4'");
/// testing::roundtrip::<ast::Lit>("b\"bytes\"");
/// testing::roundtrip::<ast::Lit>("1.2");
/// testing::roundtrip::<ast::Lit>("42");
/// testing::roundtrip::<ast::Lit>("#{\"foo\": b\"bar\"}");
/// testing::roundtrip::<ast::Lit>("Disco {\"never_died\": true }");
/// testing::roundtrip::<ast::Lit>("\"mary had a little lamb\"");
/// testing::roundtrip::<ast::Lit>("(false, 1, 'n')");
/// testing::roundtrip::<ast::Lit>("[false, 1, 'b']");
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
            K!['('] => {
                return Ok(match p.nth(1)? {
                    K![')'] => Lit::Unit(p.parse()?),
                    _ => Lit::Tuple(p.parse()?),
                });
            }
            K!['['] => return Ok(Lit::Vec(p.parse()?)),
            K![#] | K![ident] => {
                if let K!['{'] = p.nth(1)? {
                    return Ok(Lit::Object(p.parse()?));
                }
            }
            _ => (),
        }

        Err(ParseError::expected(
            &p.next()?,
            r#"expected literal like `"Hello World"` or 42"#,
        ))
    }
}
