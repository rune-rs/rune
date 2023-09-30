use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::ExprLoop>("loop {}");
    rt::<ast::ExprLoop>("loop { 1; }");
    rt::<ast::ExprLoop>("'label: loop {1;}");
    rt::<ast::ExprLoop>("#[attr] 'label: loop {x();}");
}

/// A `loop` expression.
///
/// * `loop { ... }`.
#[derive(Debug, TryClone, PartialEq, Eq, Parse, ToTokens, Spanned)]
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
