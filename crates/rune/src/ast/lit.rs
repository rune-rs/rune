use crate::ast::prelude::*;
use crate::macros::IntoLit;

/// A literal value
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
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
    /// Construct a new literal from within a macro context.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::ast;
    /// use rune::macros::MacroContext;
    ///
    /// MacroContext::test(|ctx| {
    ///     let lit = ast::Lit::new(ctx, "hello world");
    ///     assert!(matches!(lit, ast::Lit::Str(..)))
    /// });
    /// ```
    pub fn new<T>(ctx: &mut MacroContext<'_, '_>, lit: T) -> Self
    where
        T: IntoLit,
    {
        Self::new_with(lit, ctx.macro_span(), &mut ctx.q_mut().storage)
    }

    /// Construct a new literal with the specified span and storage.
    ///
    /// This does not panic outside of a macro context, but requires access to
    /// the specified arguments.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rune::ast;
    /// use rune::ast::Span;
    /// use rune::macros::Storage;
    ///
    /// let mut storage = Storage::default();
    /// let string = ast::Lit::new_with("hello world", Span::empty(), &mut storage);
    /// ```
    pub fn new_with<T>(lit: T, span: Span, storage: &mut Storage) -> Self
    where
        T: IntoLit,
    {
        T::into_lit(lit, span, storage)
    }

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
/// ```rust
/// use rune::{testing, ast};
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

        Err(ParseError::expected(
            &p.next()?,
            r#"expected literal like `"Hello World"` or 42"#,
        ))
    }
}
