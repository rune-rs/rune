use crate::ast::prelude::*;

#[test]
fn ast_parse() {
    use crate::testing::rt;

    rt::<ast::ExprFieldAccess>("foo.bar");
    rt::<ast::ExprFieldAccess>("foo.bar::<A, B>");
    rt::<ast::ExprFieldAccess>("foo.0.bar");
    // Note: tuple accesses must be disambiguated.
    rt::<ast::ExprFieldAccess>("(foo.0).1");
}

/// A field access.
///
/// * `<expr>.<field>`.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub struct ExprFieldAccess {
    /// Attributes associated with expression.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The expr where the field is being accessed.
    pub expr: Box<ast::Expr>,
    /// The parsed dot separator.
    pub dot: T![.],
    /// The field being accessed.
    pub expr_field: ExprField,
}

expr_parse!(FieldAccess, ExprFieldAccess, "field access expression");

/// The field being accessed.
#[derive(Debug, TryClone, PartialEq, Eq, ToTokens, Spanned)]
#[non_exhaustive]
pub enum ExprField {
    /// An identifier.
    Path(ast::Path),
    /// A literal number.
    LitNumber(ast::LitNumber),
}
