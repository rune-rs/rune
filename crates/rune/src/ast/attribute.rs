use crate::ast::prelude::*;

/// Attribute like `#[derive(Debug)]`
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct Attribute {
    /// The `#` character
    pub hash: T![#],
    /// Specify if the attribute is outer `#!` or inner `#`
    pub style: AttrStyle,
    /// The `[` character
    pub open: T!['['],
    /// The path of the attribute
    pub path: ast::Path,
    /// The input to the input of the attribute
    #[rune(optional)]
    pub input: TokenStream,
    /// The `]` character
    pub close: T![']'],
}

/// Parsing an Attribute
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::Attribute>("#[foo = \"foo\"]");
/// testing::roundtrip::<ast::Attribute>("#[foo()]");
/// testing::roundtrip::<ast::Attribute>("#![foo]");
/// testing::roundtrip::<ast::Attribute>("#![cfg(all(feature = \"potato\"))]");
/// testing::roundtrip::<ast::Attribute>("#[x+1]");
/// ```
impl Parse for Attribute {
    fn parse(p: &mut Parser<'_>) -> Result<Self, ParseError> {
        let hash = p.parse()?;
        let style = p.parse()?;
        let open = p.parse()?;
        let path = p.parse()?;

        let close;

        let mut level = 1;
        let mut input = TokenStream::new();

        loop {
            let token = p.next()?;

            match token.kind {
                K!['['] => level += 1,
                K![']'] => {
                    level -= 1;
                }
                _ => (),
            }

            if level == 0 {
                close = ast::CloseBracket { token };
                break;
            }

            input.push(token);
        }

        Ok(Attribute {
            hash,
            style,
            open,
            path,
            input,
            close,
        })
    }
}

impl Peek for Attribute {
    fn peek(p: &mut Peeker<'_>) -> bool {
        match (p.nth(0), p.nth(1)) {
            (K![#], K![!]) => true,
            (K![#], K!['[']) => true,
            _ => false,
        }
    }
}

impl Description for &Attribute {
    fn description(self) -> &'static str {
        match &self.style {
            AttrStyle::Inner => "inner attribute",
            AttrStyle::Outer(_) => "outer attribute",
        }
    }
}

/// Whether or not the attribute is an outer `#!` or inner `#` attribute
#[derive(Debug, Clone, Copy, PartialEq, Eq, ToTokens)]
pub enum AttrStyle {
    /// `#`
    Inner,
    /// `#!`
    Outer(T![!]),
}

impl Parse for AttrStyle {
    fn parse(p: &mut Parser) -> Result<Self, ParseError> {
        Ok(if p.peek::<T![!]>()? {
            Self::Outer(p.parse()?)
        } else {
            Self::Inner
        })
    }
}

/// Helper struct to only parse inner attributes.
pub(crate) struct InnerAttribute(pub(crate) Attribute);

impl Parse for InnerAttribute {
    fn parse(p: &mut Parser) -> Result<Self, ParseError> {
        let attribute: Attribute = p.parse()?;

        match attribute.style {
            AttrStyle::Inner => Ok(Self(attribute)),
            _ => Err(ParseError::expected(
                &attribute,
                "inner attribute like `#![allow(unused)]`",
            )),
        }
    }
}

/// Tag struct to assist peeking for an outer `#![...]` attributes at the top of
/// a module/file
pub struct OuterAttribute;

impl Peek for OuterAttribute {
    fn peek(p: &mut Peeker<'_>) -> bool {
        match (p.nth(0), p.nth(1)) {
            (K![#], K![!]) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ast;
    use crate::parsing::parse_all;
    use crate::SourceId;

    #[test]
    fn test_parse_attribute() {
        const TEST_STRINGS: &[&str] = &[
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
            parse_all::<ast::Attribute>(s, SourceId::empty()).expect(s);
            let withbang = s.replacen("#[", "#![", 1);
            parse_all::<ast::Attribute>(&withbang, SourceId::empty()).expect(&withbang);
        }
    }
}
