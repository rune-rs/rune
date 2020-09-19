use crate::ast;
use crate::ast::utils::{CloseDelim, OpenDelim};
use crate::{
    IntoTokens, MacroContext, Parse, ParseError, ParseErrorKind, Parser, Peek, Spanned, TokenStream,
};
use runestick::Span;

/// Attribute like `#[derive(Debug)]`
#[derive(Debug, Clone)]
pub struct Attribute {
    /// The `#` character
    pub hash: ast::Hash,
    /// Specify if the attribute is outer `#!` or inner `#`
    pub style: AttrStyle,
    /// The `[` character
    pub open: ast::OpenBracket,
    /// The path of the attribute
    pub path: ast::Path,
    /// The input to the input of the attribute
    pub input: TokenStream,
    //input: Option<AttrInput>,
    /// The `]` character
    pub close: ast::CloseBracket,
}

impl crate::Spanned for Attribute {
    fn span(&self) -> Span {
        self.hash.span().join(self.close.span())
    }
}

/// Parsing an Attribute
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast, ParseError};
///
/// parse_all::<ast::Attribute>("#[foo = \"foo\"]").unwrap();
/// parse_all::<ast::Attribute>("#[foo()]").unwrap();
/// parse_all::<ast::Attribute>("#![foo]").unwrap();
/// parse_all::<ast::Attribute>("#![cfg(all(feature = \"potato\"))]").unwrap();
/// assert!(parse_all::<ast::Attribute>("#[x+1]").is_err());
///
/// ```
impl Parse for Attribute {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        Ok(Attribute {
            hash: parser.parse()?,
            style: parser.parse::<Option<ast::Bang>>()?.into(),
            open: parser.parse()?,
            path: parser.parse()?,
            input: parser
                .parse::<Option<AttrInput>>()?
                .map(|input| {
                    let mut stream = TokenStream::new(vec![], input.span());
                    input.into_tokens(&mut MacroContext::empty(), &mut stream);
                    stream
                })
                .unwrap_or_else(|| TokenStream::new(vec![], Span::default())),
            // input: parser.parse()?,
            close: parser.parse()?,
        })
    }
}

impl Peek for Attribute {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        let t1 = t1.as_ref().map(|t1| t1.kind);
        let t2 = t2.as_ref().map(|t2| t2.kind);

        match (t1, t2) {
            (Some(ast::Kind::Pound), Some(ast::Kind::Bang))
            | (Some(ast::Kind::Pound), Some(ast::Kind::Open(ast::Delimiter::Bracket))) => true,
            _ => false,
        }
    }
}

impl IntoTokens for Attribute {
    fn into_tokens(&self, context: &mut MacroContext, stream: &mut TokenStream) {
        self.hash.into_tokens(context, stream);
        self.style.into_tokens(context, stream);
        self.open.into_tokens(context, stream);
        self.path.into_tokens(context, stream);
        self.input.into_tokens(context, stream);
        self.close.into_tokens(context, stream);
    }
}

/// Whether or not the attribute is an outer `#!` or inner `#` attribute
#[derive(Debug, Copy, Clone)]
pub enum AttrStyle {
    /// `#`
    Inner,
    /// `#!`
    Outer(ast::Bang),
}

impl From<Option<ast::Bang>> for AttrStyle {
    fn from(bang: Option<ast::Bang>) -> Self {
        match bang {
            Some(bang) => AttrStyle::Outer(bang),
            None => AttrStyle::Inner,
        }
    }
}

impl IntoTokens for AttrStyle {
    fn into_tokens(&self, context: &mut MacroContext, stream: &mut TokenStream) {
        match self {
            AttrStyle::Inner => {}
            AttrStyle::Outer(bang) => bang.into_tokens(context, stream),
        }
    }
}

#[derive(Debug, Clone)]
enum AttrInput {
    DelimTokenTree(DelimTokenTree),
    AssignLit(AssignLit),
}

impl Parse for AttrInput {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        if parser.peek::<AssignLit>()? {
            Ok(AttrInput::AssignLit(parser.parse()?))
        } else if parser.peek::<DelimTokenTree>()? {
            Ok(AttrInput::DelimTokenTree(parser.parse()?))
        } else {
            let token = parser.token_peek_eof()?;
            Err(ParseError::new(
                token,
                ParseErrorKind::ExpectedAttributeInput { actual: token.kind },
            ))
        }
    }
}

impl Peek for AttrInput {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        AssignLit::peek(t1, t2) || DelimTokenTree::peek(t1, t2)
    }
}

impl IntoTokens for AttrInput {
    fn into_tokens(&self, context: &mut MacroContext, stream: &mut TokenStream) {
        use AttrInput::*;

        match self {
            DelimTokenTree(value) => value.into_tokens(context, stream),
            AssignLit(value) => value.into_tokens(context, stream),
        }
    }
}

impl crate::Spanned for AttrInput {
    fn span(&self) -> Span {
        use AttrInput::*;
        match self {
            DelimTokenTree(tt) => tt.span(),
            AssignLit(lit) => lit.span(),
        }
    }
}

/// `= LiteralExpr`
#[derive(Debug, Clone)]
struct AssignLit {
    pub equal: ast::Eq,
    pub lit: ast::Lit,
}

impl Parse for AssignLit {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let equal: ast::Eq = parser.parse()?;
        let lit: ast::Lit = parser.parse()?;
        Ok(AssignLit { equal, lit })
    }
}

impl Peek for AssignLit {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        if let Some(t1) = t1 {
            t1.kind == ast::Kind::Eq && t2.is_some()
        } else {
            false
        }
    }
}

impl crate::Spanned for AssignLit {
    fn span(&self) -> Span {
        self.equal.span().join(self.lit.span())
    }
}

impl IntoTokens for AssignLit {
    fn into_tokens(&self, context: &mut MacroContext, stream: &mut TokenStream) {
        self.equal.into_tokens(context, stream);
        self.lit.into_tokens(context, stream);
    }
}

/// A token that is not a Delimiter
#[derive(Debug, Clone, Copy)]
pub struct NonDelimiter(ast::Token);

impl Parse for NonDelimiter {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        if parser.peek::<NonDelimiter>()? {
            Ok(NonDelimiter(parser.token_next()?))
        } else {
            let token = parser.token_peek_eof()?;
            Err(ParseError::new(
                token,
                ParseErrorKind::ExpectedNonDelimter { actual: token.kind },
            ))
        }
    }
}

impl Peek for NonDelimiter {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        t1.is_some() && !(OpenDelim::peek(t1, t2) || CloseDelim::peek(t1, t2))
    }
}

impl IntoTokens for NonDelimiter {
    fn into_tokens(&self, context: &mut MacroContext, stream: &mut TokenStream) {
        self.0.into_tokens(context, stream)
    }
}

/// Helper to parse a token tree as per the rust attribute spec.
///
/// ```text
/// DelimTokenTree :
/// ( TokenTree* )
/// | [ TokenTree* ]
/// | { TokenTree* }
/// ```
#[derive(Debug, Clone)]
enum TokenTree {
    Token(NonDelimiter),
    DelimTokenTree(DelimTokenTree),
}

impl Parse for TokenTree {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        if parser.peek::<NonDelimiter>()? {
            Ok(TokenTree::Token(parser.parse()?))
        } else if parser.peek::<DelimTokenTree>()? {
            Ok(TokenTree::DelimTokenTree(parser.parse()?))
        } else {
            let token = parser.token_peek_eof()?;

            Err(ParseError::new(
                token,
                ParseErrorKind::ExpectedNonDelimter { actual: token.kind },
            ))
        }
    }
}

impl Peek for TokenTree {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        NonDelimiter::peek(t1, t2) || DelimTokenTree::peek(t1, t2)
    }
}

impl IntoTokens for TokenTree {
    fn into_tokens(&self, context: &mut MacroContext, stream: &mut TokenStream) {
        use TokenTree::*;
        match self {
            Token(t) => t.into_tokens(context, stream),
            DelimTokenTree(tt) => tt.into_tokens(context, stream),
        }
    }
}

/// ```text
/// DelimTokenTree :
/// ( TokenTree* )
/// | [ TokenTree* ]
/// | { TokenTree* }
/// ```
#[derive(Debug, Clone)]
struct DelimTokenTree {
    /// The open delimiter of the TokenTree
    pub open: OpenDelim,
    /// The inner TokenTree
    pub tokentree: Vec<TokenTree>,
    /// The close delimiter which must match the open delimiter
    pub close: CloseDelim,
}

impl Parse for DelimTokenTree {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let open: OpenDelim = parser.parse()?;
        let mut tokentree = vec![];
        while parser.peek::<TokenTree>().unwrap_or(false) {
            tokentree.push(parser.parse()?);
        }

        let close: CloseDelim = parser.parse()?;

        let tokentree = DelimTokenTree {
            open,
            tokentree,
            close,
        };

        if open.delim_kind() == close.delim_kind() {
            Ok(tokentree)
        } else {
            Err(ParseError::new(
                tokentree,
                ParseErrorKind::TokenMismatch {
                    expected: ast::Kind::Close(open.delim_kind()),
                    actual: close.token().kind,
                },
            ))
        }
    }
}

impl Peek for DelimTokenTree {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        OpenDelim::peek(t1, t2) && t2.is_some()
    }
}

impl crate::Spanned for DelimTokenTree {
    fn span(&self) -> Span {
        Span {
            start: self.open.token().span().start,
            end: self.close.token().span().end,
        }
    }
}

impl IntoTokens for DelimTokenTree {
    fn into_tokens(&self, context: &mut MacroContext, stream: &mut TokenStream) {
        self.open.into_tokens(context, stream);
        for tt in self.tokentree.iter() {
            tt.into_tokens(context, stream);
        }
        self.close.into_tokens(context, stream);
    }
}

/// Tag struct to assist parsing Outer `#![...]` attributes at the top of a module/file
pub struct OuterAttribute;

impl Peek for OuterAttribute {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        let kind1 = t1.map(|t| t.kind);
        let kind2 = t2.map(|t| t.kind);

        match (kind1, kind2) {
            (Some(ast::Kind::Pound), Some(ast::Kind::Bang)) => true,
            _ => false,
        }
    }
}

#[test]
fn test_attr_input() {
    crate::parse_all::<AttrInput>("= 1").unwrap();
    crate::parse_all::<AttrInput>("= \"a\"").unwrap();
    crate::parse_all::<AttrInput>("= b\"1\"").unwrap();
    crate::parse_all::<AttrInput>("= false").unwrap();
    crate::parse_all::<AttrInput>("= [1,2,3] }").unwrap();
    crate::parse_all::<AttrInput>("= #{\"field\": [1,2,3] }").unwrap();
}

#[test]
fn test_parse_attribute() {
    const TEST_STRINGS: &[&'static str] = &[
        "#[foo]",
        "#[a::b::c]",
        "#[foo = \"hello world\"]",
        "#[foo = 1]",
        "#[foo = 1.3]",
        "#[foo = true]",
        "#[foo = b\"bytes\"]",
        "#[foo = (1, 2, \"string\")]",
        "#[foo = #{\"a\": 1} ]",
        r#"#[foo = Fred {"a": 1} ]"#,
        r#"#[foo = a::Fred {"a": #{ "b": 2 } } ]"#,
        "#[bar()]",
        "#[bar(baz)]",
        "#[derive(Debug, PartialEq, PartialOrd)]",
        "#[tracing::instrument(skip(non_debug))]",
        "#[zanzibar(a = \"z\", both = false, sasquatch::herring)]",
        r#"#[doc = "multiline \
                  docs are neat"
          ]"#,
    ];

    for s in TEST_STRINGS.iter() {
        crate::parse_all::<ast::Attribute>(s).expect(s);
        let withbang = s.replacen("#[", "#![", 1);
        crate::parse_all::<ast::Attribute>(&withbang).expect(&withbang);
    }
}
