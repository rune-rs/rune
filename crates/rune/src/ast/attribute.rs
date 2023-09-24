use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::Attribute>("#[foo = \"foo\"]");
    rt::<ast::Attribute>("#[foo()]");
    rt::<ast::Attribute>("#![foo]");
    rt::<ast::Attribute>("#![cfg(all(feature = \"potato\"))]");
    rt::<ast::Attribute>("#[x+1]");

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
        rt::<ast::Attribute>(s);
        let withbang = s.replacen("#[", "#![", 1);
        rt::<ast::Attribute>(&withbang);
    }
}

/// Attributes like:
///
/// * `#[derive(Debug)]`.
/// * `#![doc = "test"]`.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct Attribute {
    /// The `#` character
    pub hash: T![#],
    /// Specify if the attribute is outer `#!` or inner `#`
    #[rune(option)]
    pub style: AttrStyle,
    /// The `[` character
    pub open: T!['['],
    /// The path of the attribute
    pub path: ast::Path,
    /// The input to the input of the attribute
    #[rune(iter)]
    pub input: TokenStream,
    /// The `]` character
    pub close: T![']'],
}

impl Attribute {
    pub(crate) fn input_span(&self) -> Span {
        self.input
            .option_span()
            .unwrap_or_else(|| self.close.span.head())
    }
}

impl Parse for Attribute {
    fn parse(p: &mut Parser<'_>) -> Result<Self> {
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
                close = ast::CloseBracket { span: token.span };
                break;
            }

            input.push(token)?;
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

impl IntoExpectation for Attribute {
    fn into_expectation(self) -> Expectation {
        Expectation::Description(match &self.style {
            AttrStyle::Inner => "inner attribute",
            AttrStyle::Outer(_) => "outer attribute",
        })
    }
}

/// Whether or not the attribute is an outer `#!` or inner `#` attribute
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, OptionSpanned, ToTokens)]
#[try_clone(copy)]
#[non_exhaustive]
pub enum AttrStyle {
    /// `#`
    Inner,
    /// `#!`
    Outer(T![!]),
}

impl Parse for AttrStyle {
    fn parse(p: &mut Parser) -> Result<Self> {
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
    fn parse(p: &mut Parser) -> Result<Self> {
        let attribute: Attribute = p.parse()?;

        match attribute.style {
            AttrStyle::Inner => Ok(Self(attribute)),
            _ => Err(compile::Error::expected(
                attribute,
                "inner attribute like `#![allow(unused)]`",
            )),
        }
    }
}

/// Tag struct to assist peeking for an outer `#![...]` attributes at the top of
/// a module/file
#[non_exhaustive]
pub(crate) struct OuterAttribute;

impl Peek for OuterAttribute {
    fn peek(p: &mut Peeker<'_>) -> bool {
        match (p.nth(0), p.nth(1)) {
            (K![#], K![!]) => true,
            _ => false,
        }
    }
}
