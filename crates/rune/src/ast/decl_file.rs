use crate::ast::{DeclFn, DeclUse};
use crate::error::{ParseError, Result};
use crate::parser::Parser;
use crate::token::Kind;
use crate::traits::Parse;

/// A parsed file.
pub struct DeclFile {
    /// Imports for the current file.
    pub imports: Vec<DeclUse>,
    /// All function declarations in the file.
    pub functions: Vec<DeclFn>,
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
        let mut imports = Vec::new();
        let mut functions = Vec::new();

        while !parser.is_eof()? {
            match parser.token_peek()?.map(|t| t.kind) {
                Some(Kind::Use) => {
                    imports.push(parser.parse()?);
                }
                _ => {
                    functions.push(parser.parse()?);
                }
            }
        }

        Ok(Self { imports, functions })
    }
}
