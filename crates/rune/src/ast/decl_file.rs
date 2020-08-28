use crate::ast;
use crate::error::{ParseError, Result};
use crate::parser::Parser;
use crate::traits::Parse;

/// A parsed file.
pub struct DeclFile {
    /// All the declarations in a file.
    pub decls: Vec<(ast::Decl, Option<ast::SemiColon>)>,
}

/// Parse a file.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::DeclFile>(r#"
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
/// parse_all::<ast::DeclFile>(r#"
/// use http;
///
/// fn main() {
///     let client = http::client();
///     let response = client.get("https://google.com");
///     let text = response.text();
/// }
/// "#).unwrap();
/// ```
// TODO: this is a false positive: https://github.com/rust-lang/rust-clippy/issues/5879
#[allow(clippy::needless_doctest_main)]
impl Parse for DeclFile {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let mut decls = Vec::new();

        while !parser.is_eof()? {
            let decl: ast::Decl = parser.parse()?;

            let semi_colon = if decl.needs_semi_colon() || parser.peek::<ast::SemiColon>()? {
                Some(parser.parse::<ast::SemiColon>()?)
            } else {
                None
            };

            decls.push((decl, semi_colon));
        }

        Ok(Self { decls })
    }
}
