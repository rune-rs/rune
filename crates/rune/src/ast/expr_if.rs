use crate::ast::{Attribute, Condition, Else, ExprBlock, ExprElse, ExprElseIf, If};
use crate::{Parse, ParseError, Parser, Spanned, ToTokens};

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
    pub attributes: Vec<Attribute>,
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
    /// Parse an if statement attaching the given attributes
    pub fn parse_with_attributes(
        parser: &mut Parser,
        attributes: Vec<Attribute>,
    ) -> Result<Self, ParseError> {
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
            attributes,
            if_,
            condition,
            block,
            expr_else_ifs,
            expr_else,
        })
    }
}

impl Parse for ExprIf {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let attributes = parser.parse()?;
        Self::parse_with_attributes(parser, attributes)
    }
}
