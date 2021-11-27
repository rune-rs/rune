use crate::ast::prelude::*;

/// A `while` loop: `while [expr] { ... }`.
///
/// # Examples
///
/// ```
/// use rune::{ast, testing};
///
/// testing::roundtrip::<ast::ExprWhile>("while x {}");
/// testing::roundtrip::<ast::ExprWhile>("'label: while x {}");
/// testing::roundtrip::<ast::ExprWhile>("#[attr] 'label: while x {}");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned)]
#[rune(parse = "meta_only")]
#[non_exhaustive]
pub struct ExprWhile {
    /// The attributes for the `while` loop
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// A label for the while loop.
    #[rune(iter, meta)]
    pub label: Option<(ast::Label, T![:])>,
    /// The `while` keyword.
    pub while_token: T![while],
    /// The name of the binding.
    pub condition: Box<ast::Condition>,
    /// The body of the while loop.
    pub body: Box<ast::Block>,
}

expr_parse!(While, ExprWhile, "while expression");
