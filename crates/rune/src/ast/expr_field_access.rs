use crate::ast;
use crate::{Spanned, ToTokens};

/// A field access `<expr>.<field>`.
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ExprFieldAccess>("foo.bar");
/// testing::roundtrip::<ast::ExprFieldAccess>("foo.bar::<A, B>");
/// testing::roundtrip::<ast::ExprFieldAccess>("foo.0.bar");
/// // Note: tuple accesses must be disambiguated.
/// testing::roundtrip::<ast::ExprFieldAccess>("(foo.0).1");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ExprFieldAccess {
    /// Attributes associated with expression.
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The expr where the field is being accessed.
    pub expr: ast::Expr,
    /// The parsed dot separator.
    pub dot: T![.],
    /// The field being accessed.
    pub expr_field: ExprField,
}

expr_parse!(FieldAccess, ExprFieldAccess, "field access expression");

/// The field being accessed.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub enum ExprField {
    /// An identifier.
    Path(ast::Path),
    /// A literal number.
    LitNumber(ast::LitNumber),
}
