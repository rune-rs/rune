use crate::ast::prelude::*;

/// A parsed file.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens)]
#[non_exhaustive]
pub struct File {
    /// Top level "Outer" `#![...]` attributes for the file
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// All the declarations in a file.
    #[rune(iter)]
    pub items: Vec<(ast::Item, Option<T![;]>)>,
}

impl OptionSpanned for File {
    fn option_span(&self) -> Option<Span> {
        let start = self.attributes.option_span();
        let end = self.attributes.option_span();

        match (start, end) {
            (Some(start), Some(end)) => Some(start.join(end)),
            (Some(start), None) => Some(start),
            (None, Some(end)) => Some(end),
            _ => None,
        }
    }
}

/// Parse a file.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::File>(r#"
/// use foo;
///
/// fn foo() {
///     42
/// }
///
/// use bar;
///
/// fn bar(a, b) {
///     a
/// }
/// "#);
/// ```
///
/// # Realistic Example
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::File>(r#"
/// use http;
///
/// fn main() {
///     let client = http::client();
///     let response = client.get("https://google.com");
///     let text = response.text();
/// }
/// "#);
///```
///
/// # File Attributes Example
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::File>(r#"
/// // NB: Attributes are currently rejected by the compiler
/// #![feature(attributes)]
///
/// fn main() {
///     for x in [1,2,3,4,5,6] {
///         println(`{x}`)
///     }
/// }
/// "#);
/// ```
///
// TODO: this is a false positive: https://github.com/rust-lang/rust-clippy/issues/5879
#[allow(clippy::needless_doctest_main)]
impl Parse for File {
    fn parse(p: &mut Parser<'_>) -> Result<Self, ParseError> {
        let mut attributes = vec![];

        // only allow outer attributes at the top of a file
        while p.peek::<ast::attribute::OuterAttribute>()? {
            attributes.push(p.parse()?);
        }

        let mut items = Vec::new();

        let mut item_attributes = p.parse()?;
        let mut item_visibility = p.parse()?;
        let mut path = p.parse::<Option<ast::Path>>()?;

        while path.is_some() || ast::Item::peek_as_item(p.peeker()) {
            let item: ast::Item =
                ast::Item::parse_with_meta_path(p, item_attributes, item_visibility, path.take())?;

            let semi_colon = if item.needs_semi_colon() || p.peek::<T![;]>()? {
                Some(p.parse::<T![;]>()?)
            } else {
                None
            };

            items.push((item, semi_colon));
            item_attributes = p.parse()?;
            item_visibility = p.parse()?;
            path = p.parse()?;
        }

        // meta without items. maybe use different error kind?
        if let Some(span) = item_attributes.option_span() {
            return Err(ParseError::unsupported(span, "attributes"));
        }

        if let Some(span) = item_visibility.option_span() {
            return Err(ParseError::unsupported(span, "visibility"));
        }

        Ok(Self { attributes, items })
    }
}
