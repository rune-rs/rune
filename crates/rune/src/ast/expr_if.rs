use crate::ast;
use crate::{Parse, ParseError, Parser, Peek, Spanned, ToTokens};

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
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Spanned)]
pub struct ExprIf {
    /// The `attributes` of the if statement
    #[rune(iter)]
    pub attributes: Vec<ast::Attribute>,
    /// The `if` token.
    pub if_: ast::If,
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

impl ExprIf {
    /// Parse an if statement attaching the given attributes
    pub fn parse_with_attributes(
        parser: &mut Parser,
        attributes: Vec<ast::Attribute>,
    ) -> Result<Self, ParseError> {
        let if_ = parser.parse()?;
        let condition = parser.parse()?;
        let block = parser.parse()?;
        let mut expr_else_ifs = Vec::new();

        while parser.peek::<ast::Else>()? && parser.peek2::<ast::If>()? {
            expr_else_ifs.push(parser.parse()?);
        }

        let expr_else = parser.parse()?;

        Ok(ExprIf {
            attributes,
            if_,
            condition,
            block,
            expr_else_ifs,
            expr_else,
        })
    }
}

expr_parse!(ExprIf, "if expression");

/// An else branch of an if expression.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Parse, Spanned)]
pub struct ExprElseIf {
    /// The `else` token.
    pub else_: ast::Else,
    /// The `if` token.
    pub if_: ast::If,
    /// The condition for the branch.
    pub condition: ast::Condition,
    /// The body of the else statement.
    pub block: Box<ast::Block>,
}

/// An else branch of an if expression.
#[derive(Debug, Clone, PartialEq, Eq, ToTokens, Parse, Spanned)]
pub struct ExprElse {
    /// The `else` token.
    pub else_: ast::Else,
    /// The body of the else statement.
    pub block: Box<ast::Block>,
}

impl Peek for ExprElse {
    fn peek(t1: Option<ast::Token>, _: Option<ast::Token>) -> bool {
        matches!(peek!(t1).kind, ast::Kind::Else)
    }
}
