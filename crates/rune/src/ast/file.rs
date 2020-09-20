use crate::ast;
use crate::{Parse, ParseError, Parser, ToTokens};

/// A parsed file.
#[derive(Debug, Clone, ToTokens)]
pub struct File {
    /// Top level "Outer" `#![...]` attributes for the file
    pub attributes: Vec<ast::Attribute>,
    /// All the declarations in a file.
    pub items: Vec<(ast::Item, Option<ast::SemiColon>)>,
}

/// Parse a file.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::File>(r#"
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
/// "#).unwrap();
/// ```
///
/// # Realistic Example
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::File>(r#"
/// use http;
///
/// fn main() {
///     let client = http::client();
///     let response = client.get("https://google.com");
///     let text = response.text();
/// }
/// "#).unwrap();
///```
///
/// # File Attributes Example
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::File>(r#"
/// // NB: Attributes are currently ignored by the compiler
/// #![feature(attributes)]
///
/// fn main() {
///     for x in [1,2,3,4,5,6] {
///         println(`{x}`)
///     }
/// }
/// "#).unwrap();
/// ```
///
// TODO: this is a false positive: https://github.com/rust-lang/rust-clippy/issues/5879
#[allow(clippy::needless_doctest_main)]
impl Parse for File {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let mut attributes = vec![];

        // only allow outer attributes at the top of a file
        while parser.peek::<ast::attribute::OuterAttribute>()? {
            attributes.push(parser.parse()?);
        }

        let mut items = Vec::new();

        while parser.peek::<ast::Item>()? {
            let item: ast::Item = parser.parse()?;

            let semi_colon = if item.needs_semi_colon() || parser.peek::<ast::SemiColon>()? {
                Some(parser.parse::<ast::SemiColon>()?)
            } else {
                None
            };

            items.push((item, semi_colon));
        }

        Ok(Self { attributes, items })
    }
}
