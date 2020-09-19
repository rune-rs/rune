use crate::ast;
use crate::{IntoTokens, MacroContext, Parse, ParseError, ParseErrorKind, Parser, TokenStream};
use runestick::Span;

/// A literal value
#[derive(Debug, Clone)]
pub enum Lit {
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
    /// A unit literal
    Unit(ast::LitUnit),
    /// A vec literal
    Vec(ast::LitVec),
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
        let lit = if parser.peek::<ast::LitBool>()? {
            Lit::Bool(parser.parse()?)
        } else if parser.peek::<ast::LitUnit>()? {
            Lit::Unit(parser.parse()?)
        } else {
            let token = parser.token_peek_eof()?;
            match token.kind {
                ast::Kind::LitByte(_) => Lit::Byte(parser.parse()?),
                ast::Kind::LitNumber(_) => Lit::Number(parser.parse()?),
                ast::Kind::LitChar(_) => Lit::Char(parser.parse()?),
                ast::Kind::LitStr(_) => Lit::Str(parser.parse()?),
                ast::Kind::LitByteStr(_) => Lit::ByteStr(parser.parse()?),
                ast::Kind::LitTemplate(_) => Lit::Template(parser.parse()?),
                ast::Kind::Open(ast::Delimiter::Parenthesis) => Lit::Tuple(parser.parse()?),
                ast::Kind::Open(ast::Delimiter::Bracket) => Lit::Vec(parser.parse()?),
                ast::Kind::Pound | ast::Kind::Ident(_) => Lit::Object(parser.parse()?),
                _ => {
                    return Err(ParseError::new(
                        token,
                        ParseErrorKind::ExpectedLit { actual: token.kind },
                    ));
                }
            }
        };

        Ok(lit)
    }
}

impl IntoTokens for Lit {
    fn into_tokens(&self, context: &mut MacroContext, stream: &mut TokenStream) {
        use Lit::*;

        match self {
            Bool(lit) => lit.into_tokens(context, stream),
            Byte(lit) => lit.into_tokens(context, stream),
            ByteStr(lit) => lit.into_tokens(context, stream),
            Char(lit) => lit.into_tokens(context, stream),
            Number(lit) => lit.into_tokens(context, stream),
            Object(lit) => lit.into_tokens(context, stream),
            Str(lit) => lit.into_tokens(context, stream),
            Template(lit) => lit.into_tokens(context, stream),
            Tuple(lit) => lit.into_tokens(context, stream),
            Unit(lit) => lit.into_tokens(context, stream),
            Vec(lit) => lit.into_tokens(context, stream),
        }
    }
}

impl crate::Spanned for Lit {
    fn span(&self) -> Span {
        use Lit::*;

        match self {
            Bool(lit) => lit.span(),
            Byte(lit) => lit.span(),
            ByteStr(lit) => lit.span(),
            Char(lit) => lit.span(),
            Number(lit) => lit.span(),
            Object(lit) => lit.span(),
            Str(lit) => lit.span(),
            Template(lit) => lit.span(),
            Tuple(lit) => lit.span(),
            Unit(lit) => lit.span(),
            Vec(lit) => lit.span(),
        }
    }
}
