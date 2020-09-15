use crate::ast::{Condition, Else, ExprBlock, ExprElse, ExprElseIf, If};
use crate::{Parse, ParseError, Parser, Spanned};
use runestick::Span;

/// An if expression.
#[derive(Debug, Clone)]
pub struct ExprIf {
    /// The `if` token.
    pub if_: If,
    /// The condition to the if statement.
    pub condition: Condition,
    /// The body of the if statement.
    pub block: Box<ExprBlock>,
    /// Else if branches.
    pub expr_else_ifs: Vec<ExprElseIf>,
    /// The else part of the if expression.
    pub expr_else: Option<ExprElse>,
}

into_tokens!(ExprIf {
    if_,
    condition,
    block,
    expr_else_ifs,
    expr_else
});

impl ExprIf {
    /// An if statement evaluates to empty if it does not have an else branch.
    pub fn produces_nothing(&self) -> bool {
        self.expr_else.is_none()
    }
}

impl Spanned for ExprIf {
    fn span(&self) -> Span {
        if let Some(else_) = &self.expr_else {
            self.if_.token.span().join(else_.block.span())
        } else if let Some(else_if) = self.expr_else_ifs.last() {
            self.if_.token.span().join(else_if.block.span())
        } else {
            self.if_.token.span().join(self.block.span())
        }
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
