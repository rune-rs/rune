use crate::{ast, Peek};
use crate::{IntoTokens, Parse, ParseError, ParseErrorKind, Parser, Resolve, Spanned, Storage};
use runestick::{Source, Span};
use std::borrow::Cow;

/// A literal object identifier.
#[derive(Debug, Clone)]
pub enum LitObjectIdent {
    /// An anonymous object.
    Anonymous(ast::Hash),
    /// A named object.
    Named(ast::Path),
}

impl LitObjectIdent {
    /// Get the span of the identifier.
    pub fn span(&self) -> Span {
        match self {
            Self::Anonymous(hash) => hash.span(),
            Self::Named(path) => path.span(),
        }
    }
}

impl Parse for LitObjectIdent {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let token = parser.token_peek_eof()?;

        Ok(match token.kind {
            ast::Kind::Pound => Self::Anonymous(parser.parse()?),
            _ => Self::Named(parser.parse()?),
        })
    }
}

impl IntoTokens for LitObjectIdent {
    fn into_tokens(&self, context: &mut crate::MacroContext, stream: &mut crate::TokenStream) {
        match self {
            LitObjectIdent::Anonymous(hash) => hash.into_tokens(context, stream),
            LitObjectIdent::Named(path) => path.into_tokens(context, stream),
        }
    }
}

/// A literal object field.
#[derive(Debug, Clone)]
pub struct LitObjectFieldAssign {
    /// The key of the field.
    pub key: LitObjectKey,
    /// The assigned expression of the field.
    pub assign: Option<(ast::Colon, ast::Expr)>,
}

impl LitObjectFieldAssign {
    /// Get the span of the assignment.
    pub fn span(&self) -> Span {
        if let Some((_, expr)) = &self.assign {
            self.key.span().join(expr.span())
        } else {
            self.key.span()
        }
    }

    /// Check if assignment is constant or not.
    pub fn is_const(&self) -> bool {
        match &self.assign {
            Some((_, expr)) => expr.is_const(),
            None => false,
        }
    }
}

/// Parse an object literal.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::LitObjectFieldAssign>("\"foo\": 42").unwrap();
/// parse_all::<ast::LitObjectFieldAssign>("\"foo\": 42").unwrap();
/// parse_all::<ast::LitObjectFieldAssign>("\"foo\": 42").unwrap();
/// ```
impl Parse for LitObjectFieldAssign {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let key = parser.parse()?;

        let assign = if parser.peek::<ast::Colon>()? {
            let colon = parser.parse()?;
            let expr = parser.parse::<ast::Expr>()?;
            Some((colon, expr))
        } else {
            None
        };

        Ok(Self { key, assign })
    }
}

impl IntoTokens for LitObjectFieldAssign {
    fn into_tokens(&self, context: &mut crate::MacroContext, stream: &mut crate::TokenStream) {
        self.key.into_tokens(context, stream);
        self.assign.into_tokens(context, stream);
    }
}

/// Possible literal object keys.
#[derive(Debug, Clone)]
pub enum LitObjectKey {
    /// A literal string (with escapes).
    LitStr(ast::LitStr),
    /// An identifier.
    Ident(ast::Ident),
}

impl LitObjectKey {
    /// Get the span of the object key.
    pub fn span(&self) -> Span {
        match self {
            Self::LitStr(lit_str) => lit_str.span(),
            Self::Ident(ident) => ident.span(),
        }
    }
}

/// Parse an object literal.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::LitObjectKey>("foo").unwrap();
/// parse_all::<ast::LitObjectKey>("\"foo \\n bar\"").unwrap();
/// ```
impl Parse for LitObjectKey {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let token = parser.token_peek_eof()?;

        Ok(match token.kind {
            ast::Kind::LitStr { .. } => Self::LitStr(parser.parse()?),
            ast::Kind::Ident(..) => Self::Ident(parser.parse()?),
            _ => {
                return Err(ParseError::new(
                    token,
                    ParseErrorKind::ExpectedLitObjectKey { actual: token.kind },
                ));
            }
        })
    }
}

impl<'a> Resolve<'a> for LitObjectKey {
    type Output = Cow<'a, str>;

    fn resolve(&self, storage: &Storage, source: &'a Source) -> Result<Self::Output, ParseError> {
        Ok(match self {
            Self::LitStr(lit_str) => lit_str.resolve(storage, source)?,
            Self::Ident(ident) => ident.resolve(storage, source)?,
        })
    }
}

impl IntoTokens for LitObjectKey {
    fn into_tokens(&self, context: &mut crate::MacroContext, stream: &mut crate::TokenStream) {
        match self {
            LitObjectKey::LitStr(s) => s.into_tokens(context, stream),
            LitObjectKey::Ident(ident) => ident.into_tokens(context, stream),
        }
    }
}

/// A number literal.
#[derive(Debug, Clone)]
pub struct LitObject {
    /// An object identifier.
    pub ident: LitObjectIdent,
    /// The open bracket.
    pub open: ast::OpenBrace,
    /// Items in the object declaration.
    pub assignments: Vec<LitObjectFieldAssign>,
    /// The close bracket.
    pub close: ast::CloseBrace,
    /// Indicates if the object is completely literal and cannot have side
    /// effects.
    is_const: bool,
}

impl LitObject {
    /// Test if the entire expression is constant.
    pub fn is_const(&self) -> bool {
        self.is_const
    }

    /// Parse a literal object with the given path.
    pub fn parse_with_ident(
        parser: &mut Parser<'_>,
        ident: ast::LitObjectIdent,
    ) -> Result<Self, ParseError> {
        let open = parser.parse()?;

        let mut assignments = Vec::new();

        let mut is_const = true;

        while !parser.peek::<ast::CloseBrace>()? {
            let assign = parser.parse::<LitObjectFieldAssign>()?;

            if !assign.is_const() {
                is_const = false;
            }

            assignments.push(assign);

            if parser.peek::<ast::Comma>()? {
                parser.parse::<ast::Comma>()?;
            } else {
                break;
            }
        }

        let close = parser.parse()?;

        Ok(Self {
            ident,
            open,
            assignments,
            close,
            is_const,
        })
    }
}

impl Spanned for LitObject {
    fn span(&self) -> Span {
        self.ident.span().join(self.close.span())
    }
}

/// Parse an object literal.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::LitObject>("Foo {\"foo\": 42}").unwrap();
/// parse_all::<ast::LitObject>("#{\"foo\": 42}").unwrap();
/// parse_all::<ast::LitObject>("#{\"foo\": 42,}").unwrap();
/// ```
impl Parse for LitObject {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let ident = parser.parse()?;
        Self::parse_with_ident(parser, ident)
    }
}

impl IntoTokens for LitObject {
    fn into_tokens(&self, context: &mut crate::MacroContext, stream: &mut crate::TokenStream) {
        self.ident.into_tokens(context, stream);
        self.open.into_tokens(context, stream);

        for assign in &self.assignments {
            assign.into_tokens(context, stream);
        }

        self.close.into_tokens(context, stream);
    }
}

impl Peek for LitObject {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        let (t1, t2) = match (t1, t2) {
            (Some(t1), Some(t2)) => (t1, t2),
            _ => return false,
        };
        match (t1.kind, t2.kind) {
            (ast::Kind::Ident(_), ast::Kind::Open(ast::Delimiter::Brace))
            | (ast::Kind::Pound, ast::Kind::Open(ast::Delimiter::Brace)) => true,
            _ => false,
        }
    }
}

/// A tag object to help peeking for anonymous object case to help
/// differentiate anonymous objects and attributes when parsing block
/// expressions.
pub struct AnonymousLitObject;

impl Peek for AnonymousLitObject {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        let kind1 = t1.map(|t| t.kind);
        let kind2 = t2.map(|t| t.kind);

        match (kind1, kind2) {
            (Some(ast::Kind::Pound), Some(ast::Kind::Open(ast::Delimiter::Brace))) => true,
            _ => false,
        }
    }
}
