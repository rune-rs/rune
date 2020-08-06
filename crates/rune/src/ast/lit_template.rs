use crate::ast::utils;
use crate::ast::Expr;
use crate::error::{ParseError, Result};
use crate::parser::Parser;
use crate::source::Source;
use crate::token::{Kind, Token};
use crate::traits::{Parse, Resolve};
use stk::unit::Span;

/// A string literal.
#[derive(Debug, Clone)]
pub struct LitTemplate {
    /// The token corresponding to the literal.
    token: Token,
    /// If the string literal is escaped.
    escaped: bool,
}

impl LitTemplate {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.token.span
    }
}

/// A single template component.
#[derive(Debug)]
pub enum TemplateComponent {
    /// A literal string.
    String(String),
    /// An expression inside of the template. Like `{1 + 2}`.
    Expr(Expr),
}

/// A resolved and parsed string template.
#[derive(Debug)]
pub struct Template {
    pub(crate) has_expansions: bool,
    pub(crate) size_hint: usize,
    pub(crate) components: Vec<TemplateComponent>,
}

impl<'a> Resolve<'a> for LitTemplate {
    type Output = Template;

    fn resolve(&self, source: Source<'a>) -> Result<Self::Output, ParseError> {
        let span = self.span().narrow(1);
        let string = source.source(span)?;

        let mut it = string
            .char_indices()
            .map(|(n, c)| (span.start + n, c))
            .peekable();

        let mut has_expansions = false;
        let mut size_hint = 0;
        let mut buf = String::new();

        let mut components = Vec::new();

        while let Some((_, c)) = it.next() {
            match c {
                '\\' => {
                    let c = utils::parse_char_escape(span, &mut it, utils::WithBrace(true))?;
                    buf.push(c);
                }
                '}' => {
                    return Err(ParseError::UnexpectedCloseBrace { span });
                }
                '{' => {
                    if !buf.is_empty() {
                        size_hint += buf.len();
                        components.push(TemplateComponent::String(buf.clone()));
                        buf.clear();
                    }

                    let span = utils::template_expr(span, &mut it)?;
                    let source = &source.as_str()[..span.end];

                    let mut parser = Parser::new_with_start(source, span.start);
                    let expr = Expr::parse(&mut parser)?;
                    components.push(TemplateComponent::Expr(expr));
                    has_expansions = true;
                }
                c => {
                    buf.push(c);
                }
            }
        }

        if !buf.is_empty() {
            size_hint += buf.len();
            components.push(TemplateComponent::String(buf.clone()));
            buf.clear();
        }

        Ok(Template {
            has_expansions,
            size_hint,
            components,
        })
    }
}

/// Parse a string literal.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// # fn main() -> anyhow::Result<()> {
/// parse_all::<ast::LitTemplate>("`hello world`")?;
/// parse_all::<ast::LitTemplate>("`hello\\n world`")?;
/// # Ok(())
/// # }
/// ```
impl Parse for LitTemplate {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        match token.kind {
            Kind::LitTemplate { escaped } => Ok(LitTemplate { token, escaped }),
            _ => Err(ParseError::ExpectedStringError {
                actual: token.kind,
                span: token.span,
            }),
        }
    }
}
