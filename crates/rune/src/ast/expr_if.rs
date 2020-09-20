use crate::ast::{Condition, Else, ExprBlock, ExprElse, ExprElseIf, If};
use crate::{Parse, ParseError, Parser, Spanned, ToTokens};

/// An if expression.
#[derive(Debug, Clone, ToTokens, Spanned)]
pub struct ExprIf {
    /// The `if` token.
    pub if_: If,
    /// The condition to the if statement.
    pub condition: Condition,
    /// The body of the if statement.
    pub block: Box<ExprBlock>,
    /// Else if branches.
    #[rune(iter)]
    pub expr_else_ifs: Vec<ExprElseIf>,
    /// The else part of the if expression.
    #[rune(iter)]
    pub expr_else: Option<ExprElse>,
}

impl ExprIf {
    /// An if statement evaluates to empty if it does not have an else branch.
    pub fn produces_nothing(&self) -> bool {
        self.expr_else.is_none()
    }
}

/// Parse an if statement.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// parse_all::<ast::ExprIf>("if 0 {  }").unwrap();
/// parse_all::<ast::ExprIf>("if 0 {  } else {  }").unwrap();
/// parse_all::<ast::ExprIf>("if 0 {  } else if 0 {  } else {  }").unwrap();
/// parse_all::<ast::ExprIf>("if let v = v {  }").unwrap();
/// ```
impl Parse for ExprIf {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let if_ = parser.parse()?;
        let condition = parser.parse()?;
        let block = Box::new(parser.parse()?);
        let mut expr_else_ifs = Vec::new();
        let mut expr_else = None;

        while parser.peek::<Else>()? {
            if parser.peek2::<If>()? {
                expr_else_ifs.push(parser.parse()?);
                continue;
            }

            expr_else = Some(parser.parse()?);
        }

        Ok(ExprIf {
            if_,
            condition,
            block,
            expr_else_ifs,
            expr_else,
        })
    }
}
