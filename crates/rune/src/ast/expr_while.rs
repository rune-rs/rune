use crate::ast;
use crate::{Parse, Spanned, ToTokens};

/// A `while` loop: `while [expr] { ... }`.
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ExprWhile>("while x {}");
/// testing::roundtrip::<ast::ExprWhile>("'label: while x {}");
/// testing::roundtrip::<ast::ExprWhile>("#[attr] 'label: while x {}");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned)]
#[rune(parse = "meta_only")]
pub struct ExprWhile {
    /// The attributes for the `while` loop
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// A label for the while loop.
    #[rune(iter, meta)]
    pub label: Option<(ast::Label, ast::Colon)>,
    /// The `while` keyword.
    pub while_token: ast::While,
    /// The name of the binding.
    pub condition: ast::Condition,
    /// The body of the while loop.
    pub body: Box<ast::Block>,
}

expr_parse!(ExprWhile, "while expression");
