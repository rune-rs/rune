use crate::ast;
use crate::{Parse, ParseError, ParseErrorKind, Parser, Spanned, ToTokens};

/// A literal value
#[derive(Debug, Clone, ToTokens, Spanned)]
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
    /// Test if this literal is constant.
    pub fn is_const(&self) -> bool {
        match self {
            Self::Template(..) => false,
            Self::Unit(..) => true,
            Self::Bool(..) => true,
            Self::Byte(..) => true,
            Self::Char(..) => true,
            Self::Number(..) => true,
            Self::Str(..) => true,
            Self::ByteStr(..) => true,
            Self::Vec(vec) => vec.is_const(),
            Self::Object(object) => object.is_const(),
            Self::Tuple(tuple) => tuple.is_const(),
        }
    }

    /// Test if this is an immediate literal in an expression.
    ///
    /// Here we only test for unambiguous literals which will not be caused by
    /// a later stage as an expression is being parsed.
    ///
    /// These include:
    /// * Object literals that start with a path (handled in [ast::Expr::parse_ident_start]).
    /// * Tuple literals that start with a path (handled in [ast::Expr::parse_open_paren]).
    pub(crate) fn peek_in_expr(parser: &mut Parser<'_>) -> Result<bool, ParseError> {
        let t1 = parser.token_peek()?;

        let t1 = match t1 {
            Some(t1) => t1,
            None => return Ok(false),
        };

        Ok(match t1.kind {
            ast::Kind::True | ast::Kind::False => true,
            ast::Kind::LitByte(_) => true,
            ast::Kind::LitNumber(_) => true,
            ast::Kind::LitChar(_) => true,
            ast::Kind::LitStr(_) => true,
            ast::Kind::LitByteStr(_) => true,
            ast::Kind::LitTemplate(_) => true,
            ast::Kind::Open(ast::Delimiter::Bracket) => true,
            ast::Kind::Pound => match parser.token_peek2()?.map(|t| t.kind) {
                Some(ast::Kind::Open(ast::Delimiter::Brace)) => true,
                _ => false,
            },
            _ => false,
        })
    }
}

/// Parsing a Lit
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::Lit>("true").unwrap();
/// parse_all::<ast::Lit>("'🔥'").unwrap();
/// parse_all::<ast::Lit>("b'4'").unwrap();
/// parse_all::<ast::Lit>("b\"bytes\"").unwrap();
/// parse_all::<ast::Lit>("1.2").unwrap();
/// parse_all::<ast::Lit>("#{\"foo\": b\"bar\"}").unwrap();
/// parse_all::<ast::Lit>("Disco {\"never_died\": true }").unwrap();
/// parse_all::<ast::Lit>("\"mary had a little lamb\"").unwrap();
/// parse_all::<ast::Lit>("`{taco_tuesday}`").unwrap();
/// parse_all::<ast::Lit>("(false, 1, 'n')").unwrap();
/// parse_all::<ast::Lit>("()").unwrap();
/// parse_all::<ast::Lit>("[false, 1, 'b']").unwrap();
/// ```
impl Parse for Lit {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_peek_eof()?;

        // breaks are used as control flow to error.
        #[allow(clippy::never_loop)]
        loop {
            return Ok(match token.kind {
                ast::Kind::True | ast::Kind::False => Lit::Bool(parser.parse()?),
                ast::Kind::LitByte(_) => Lit::Byte(parser.parse()?),
                ast::Kind::LitNumber(_) => Lit::Number(parser.parse()?),
                ast::Kind::LitChar(_) => Lit::Char(parser.parse()?),
                ast::Kind::LitStr(_) => Lit::Str(parser.parse()?),
                ast::Kind::LitByteStr(_) => Lit::ByteStr(parser.parse()?),
                ast::Kind::LitTemplate(_) => Lit::Template(parser.parse()?),
                ast::Kind::Open(ast::Delimiter::Parenthesis) => {
                    match parser.token_peek2_eof()?.kind {
                        ast::Kind::Close(ast::Delimiter::Parenthesis) => Lit::Unit(parser.parse()?),
                        _ => Lit::Tuple(parser.parse()?),
                    }
                }
                ast::Kind::Open(ast::Delimiter::Bracket) => Lit::Vec(parser.parse()?),
                ast::Kind::Pound | ast::Kind::Ident(..) => match parser.token_peek2_eof()?.kind {
                    ast::Kind::Open(ast::Delimiter::Brace) => Lit::Object(parser.parse()?),
                    _ => break,
                },
                _ => break,
            });
        }

        Err(ParseError::new(
            token,
            ParseErrorKind::ExpectedLit { actual: token.kind },
        ))
    }
}
