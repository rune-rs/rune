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
    /// A byte string literal
    ByteStr(ast::LitByteStr),
    /// A character literal
    Char(ast::LitChar),
    /// A number literal
    Number(ast::LitNumber),
    /// An object literal
    Object(ast::LitObject),
    /// A string literal
    Str(ast::LitStr),
    /// A template literal
    Template(ast::LitTemplate),
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
            ast::Kind::LitByte(_) => true,
            ast::Kind::LitNumber(_) => true,
            ast::Kind::LitChar(_) => true,
            ast::Kind::LitStr(_) => true,
            ast::Kind::LitByteStr(_) => true,
            K![template] => true,
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
/// testing::roundtrip::<ast::Lit>("true");
/// testing::roundtrip::<ast::Lit>("'🔥'");
/// testing::roundtrip::<ast::Lit>("b'4'");
/// testing::roundtrip::<ast::Lit>("b\"bytes\"");
/// testing::roundtrip::<ast::Lit>("1.2");
/// testing::roundtrip::<ast::Lit>("#{\"foo\": b\"bar\"}");
/// testing::roundtrip::<ast::Lit>("Disco {\"never_died\": true }");
/// testing::roundtrip::<ast::Lit>("\"mary had a little lamb\"");
/// testing::roundtrip::<ast::Lit>("`{taco_tuesday}`");
/// testing::roundtrip::<ast::Lit>("(false, 1, 'n')");
/// testing::roundtrip::<ast::Lit>("()");
/// testing::roundtrip::<ast::Lit>("[false, 1, 'b']");
/// ```
impl Parse for Lit {
    fn parse(p: &mut Parser<'_>) -> Result<Self, ParseError> {
        match p.nth(0)? {
            K![true] | K![false] => return Ok(Lit::Bool(p.parse()?)),
            ast::Kind::LitByte(_) => return Ok(Lit::Byte(p.parse()?)),
            ast::Kind::LitNumber(_) => return Ok(Lit::Number(p.parse()?)),
            ast::Kind::LitChar(_) => return Ok(Lit::Char(p.parse()?)),
            ast::Kind::LitStr(_) => return Ok(Lit::Str(p.parse()?)),
            ast::Kind::LitByteStr(_) => return Ok(Lit::ByteStr(p.parse()?)),
            K![template] => return Ok(Lit::Template(p.parse()?)),
            K!['('] => {
                return Ok(match p.nth(1)? {
                    K![')'] => Lit::Unit(p.parse()?),
                    _ => Lit::Tuple(p.parse()?),
                });
            }
            K!['['] => return Ok(Lit::Vec(p.parse()?)),
            K![#] | K![ident(..)] => match p.nth(1)? {
                K!['{'] => return Ok(Lit::Object(p.parse()?)),
                _ => (),
            },
            _ => (),
        }

        Err(ParseError::expected(
            p.next()?,
            r#"expected literal like `"Hello World"` or 42"#,
        ))
    }
}
