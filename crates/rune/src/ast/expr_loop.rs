use crate::ast::prelude::*;

/// A `loop` expression: `loop { ... }`.
///
/// # Examples
///
/// ```
/// use rune::{ast, testing};
///
/// testing::roundtrip::<ast::ExprLoop>("loop {}");
/// testing::roundtrip::<ast::ExprLoop>("loop { 1; }");
/// testing::roundtrip::<ast::ExprLoop>("'label: loop {1;}");
/// testing::roundtrip::<ast::ExprLoop>("#[attr] 'label: loop {x();}");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned)]
#[rune(parse = "meta_only")]
#[non_exhaustive]
pub struct ExprLoop {
    /// The attributes for the `loop`
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// A label followed by a colon.
    #[rune(iter, meta)]
    pub label: Option<(ast::Label, T![:])>,
    /// The `loop` keyword.
    pub loop_token: T![loop],
    /// The body of the loop.
    pub body: Box<ast::Block>,
}

expr_parse!(Loop, ExprLoop, "loop expression");
