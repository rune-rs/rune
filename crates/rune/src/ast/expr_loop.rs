use crate::ast;
use crate::{Parse, Spanned, ToTokens};

/// A `loop` expression: `loop { ... }`.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ExprLoop>("loop {}");
/// testing::roundtrip::<ast::ExprLoop>("loop { 1; }");
/// testing::roundtrip::<ast::ExprLoop>("'label: loop {1;}");
/// testing::roundtrip::<ast::ExprLoop>("#[attr] 'label: loop {x();}");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned)]
#[rune(parse = "meta_only")]
pub struct ExprLoop {
    /// The attributes for the `loop`
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// A label followed by a colon.
    #[rune(iter, meta)]
    pub label: Option<(ast::Label, ast::Colon)>,
    /// The `loop` keyword.
    pub loop_: ast::Loop,
    /// The body of the loop.
    pub body: Box<ast::Block>,
}

expr_parse!(ExprLoop, "loop expression");
