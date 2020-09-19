use crate::ast;
use crate::{IntoTokens, MacroContext, Parse, ParseError, Parser, Peek, Spanned, TokenStream};
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
    /// The `]` character
    pub close: ast::CloseBracket,
}

into_tokens!(Attribute {
    hash,
    style,
    open,
    path,
    input,
    close,
});

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
/// parse_all::<ast::Attribute>("#[x+1]").unwrap();
/// ```
impl Parse for Attribute {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let hash = parser.parse()?;
        let style = parser.parse()?;
        let open = parser.parse()?;
        let path = parser.parse()?;

        let close;

        let mut level = 1;
        let mut stream = Vec::new();
        let end;

        loop {
            let token = parser.token_next()?;

            match token.kind {
                ast::Kind::Open(ast::Delimiter::Bracket) => level += 1,
                ast::Kind::Close(ast::Delimiter::Bracket) => {
                    level -= 1;
                }
                _ => (),
            }

            if level == 0 {
                end = Span::point(token.span().start);
                close = ast::CloseBracket { token };
                break;
            }

            stream.push(token);
        }

        Ok(Attribute {
            hash,
            style,
            open,
            path,
            input: TokenStream::new(stream, end),
            close,
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

/// Whether or not the attribute is an outer `#!` or inner `#` attribute
#[derive(Debug, Copy, Clone)]
pub enum AttrStyle {
    /// `#`
    Inner,
    /// `#!`
    Outer(ast::Bang),
}

impl IntoTokens for AttrStyle {
    fn into_tokens(&self, context: &mut MacroContext, stream: &mut TokenStream) {
        match self {
            AttrStyle::Inner => (),
            AttrStyle::Outer(bang) => bang.into_tokens(context, stream),
        }
    }
}

impl Parse for AttrStyle {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(if parser.peek::<ast::Bang>()? {
            Self::Outer(parser.parse()?)
        } else {
            Self::Inner
        })
    }
}

/// Tag struct to assist peeking for an outer `#![...]` attributes at the top of
/// a module/file
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
