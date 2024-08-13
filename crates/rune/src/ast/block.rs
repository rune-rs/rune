use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    let expr = rt::<ast::ExprBlock>("{}");
    assert_eq!(expr.block.statements.len(), 0);

    let expr = rt::<ast::ExprBlock>("{ 42 }");
    assert_eq!(expr.block.statements.len(), 1);

    let block = rt::<ast::Block>("{ foo }");
    assert_eq!(block.statements.len(), 1);

    let block = rt::<ast::Block>("{ foo; }");
    assert_eq!(block.statements.len(), 1);

    let expr = rt::<ast::ExprBlock>("#[retry] { 42 }");
    assert_eq!(expr.block.statements.len(), 1);
    assert_eq!(expr.attributes.len(), 1);

    let block = rt::<ast::Block>(
        r#"
        {
            let foo = 42;
            let bar = "string";
            baz
        }
    "#,
    );

    assert_eq!(block.statements.len(), 3);

    let block = rt::<ast::EmptyBlock>(
        r#"
        let foo = 42;
        let bar = "string";
        baz
        "#,
    );

    assert_eq!(block.statements.len(), 3);
}

/// A block of statements.
///
/// * `{ (<stmt>)* }`.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct Block {
    /// The close brace.
    pub open: T!['{'],
    /// Statements in the block.
    #[rune(iter)]
    pub statements: Vec<ast::Stmt>,
    /// The close brace.
    pub close: T!['}'],
    /// The unique identifier for the block expression.
    #[rune(skip)]
    pub(crate) id: ItemId,
}

impl Parse for Block {
    fn parse(parser: &mut Parser<'_>) -> Result<Self> {
        let mut statements = Vec::new();

        let open = parser.parse()?;

        while !parser.peek::<T!['}']>()? {
            statements.try_push(parser.parse()?)?;
        }

        let close = parser.parse()?;

        Ok(Self {
            open,
            statements,
            close,
            id: ItemId::ROOT,
        })
    }
}

/// A block of statements.
///
/// * `{ (<stmt>)* }`.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens)]
#[non_exhaustive]
pub struct EmptyBlock {
    /// Statements in the block.
    pub statements: Vec<ast::Stmt>,
    /// The unique identifier for the block expression.
    #[rune(skip)]
    pub(crate) id: ItemId,
}

impl Parse for EmptyBlock {
    fn parse(parser: &mut Parser<'_>) -> Result<Self> {
        let mut statements = Vec::new();

        while !parser.is_eof()? {
            statements.try_push(parser.parse()?)?;
        }

        Ok(Self {
            statements,
            id: ItemId::ROOT,
        })
    }
}
