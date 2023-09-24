use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    let expr = rt::<ast::ExprBlock>("async {}");
    assert_eq!(expr.block.statements.len(), 0);

    let expr = rt::<ast::ExprBlock>("async move {}");
    assert_eq!(expr.block.statements.len(), 0);

    let expr = rt::<ast::ExprBlock>("const {}");
    assert_eq!(expr.block.statements.len(), 0);

    let expr = rt::<ast::ExprBlock>("async { 42 }");
    assert_eq!(expr.block.statements.len(), 1);

    let expr = rt::<ast::ExprBlock>("#[retry] async { 42 }");
    assert_eq!(expr.block.statements.len(), 1);
    assert_eq!(expr.attributes.len(), 1);
}

/// A block expression.
///
/// * `<block>`.
/// * `async <block>`.
/// * `const <block>`.
#[derive(Debug, TryClone, PartialEq, Eq, Parse, ToTokens, Spanned)]
#[rune(parse = "meta_only")]
#[non_exhaustive]
pub struct ExprBlock {
    /// The attributes for the block.
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// The optional async token.
    #[rune(iter, meta)]
    pub async_token: Option<T![async]>,
    /// The optional const token.
    #[rune(iter, meta)]
    pub const_token: Option<T![const]>,
    /// The optional move token.
    #[rune(iter, meta)]
    pub move_token: Option<T![move]>,
    /// The close brace.
    pub block: ast::Block,
}

expr_parse!(Block, ExprBlock, "block expression");
