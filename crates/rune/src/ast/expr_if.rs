use crate::ast;
use crate::{Parse, Peek, Peeker, Spanned, ToTokens};

/// An if statement: `if cond { true } else { false }`
///
/// # Examples
///
/// ```rust
/// use rune::{testing, ast};
///
/// testing::roundtrip::<ast::ExprIf>("if 0 {  }");
/// testing::roundtrip::<ast::ExprIf>("if 0 {  } else {  }");
/// testing::roundtrip::<ast::ExprIf>("if 0 {  } else if 0 {  } else {  }");
/// testing::roundtrip::<ast::ExprIf>("if let v = v {  }");
/// testing::roundtrip::<ast::ExprIf>("#[attr] if 1 {} else {}");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Parse, ToTokens, Spanned)]
#[rune(parse = "meta_only")]
pub struct ExprIf {
    /// The `attributes` of the if statement
    #[rune(iter, meta)]
    pub attributes: Vec<ast::Attribute>,
    /// The `if` token.
    pub if_: T![if],
    /// The condition to the if statement.
    pub condition: ast::Condition,
    /// The body of the if statement.
    pub block: Box<ast::Block>,
    /// Else if branches.
    #[rune(iter)]
    pub expr_else_ifs: Vec<ExprElseIf>,
    /// The else part of the if expression.
    #[rune(iter)]
    pub expr_else: Option<ExprElse>,
}

expr_parse!(If, ExprIf, "if expression");

/// An else branch of an if expression.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Parse, Spanned)]
pub struct ExprElseIf {
    /// The `else` token.
    pub else_: T![else],
    /// The `if` token.
    pub if_: T![if],
    /// The condition for the branch.
    pub condition: ast::Condition,
    /// The body of the else statement.
    pub block: Box<ast::Block>,
}

impl Peek for ExprElseIf {
    fn peek(p: &mut Peeker<'_>) -> bool {
        matches!((p.nth(0), p.nth(1)), (K![else], K![if]))
    }
}

/// An else branch of an if expression.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Parse, Spanned)]
pub struct ExprElse {
    /// The `else` token.
    pub else_: T![else],
    /// The body of the else statement.
    pub block: Box<ast::Block>,
}

impl Peek for ExprElse {
    fn peek(p: &mut Peeker<'_>) -> bool {
        matches!(p.nth(0), K![else])
    }
}
