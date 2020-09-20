use crate::ast;
use crate::{Parse, ParseError, ParseErrorKind, Parser, Resolve, Spanned, Storage, ToTokens};
use runestick::{Source, Span};
use std::borrow::Cow;

/// A string literal.
#[derive(Debug, Clone, ToTokens, Spanned)]
pub struct LitTemplate {
    /// The token corresponding to the literal.
    token: ast::Token,
    /// The source string of the literal template.
    #[rune(skip)]
    source: ast::LitStrSource,
}

/// A single template component.
#[derive(Debug, Clone)]
pub enum TemplateComponent {
    /// A literal string.
    String(String),
    /// An expression inside of the template. Like `{1 + 2}`.
    Expr(Box<ast::Expr>),
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

    fn resolve(&self, storage: &Storage, source: &'a Source) -> Result<Self::Output, ParseError> {
        let span = self.token.span();

        let (span, text) = match self.source {
            ast::LitStrSource::Text(..) => {
                let span = span.narrow(1);

                let string = source
                    .source(span)
                    .ok_or_else(|| ParseError::new(span, ParseErrorKind::BadSlice))?;

                (span, Cow::Borrowed(string))
            }
            ast::LitStrSource::Synthetic(id) => {
                let string = storage.get_string(id).ok_or_else(|| {
                    ParseError::new(
                        span,
                        ParseErrorKind::BadSyntheticId {
                            kind: "template string",
                            id,
                        },
                    )
                })?;

                (Span::new(0, string.len()), Cow::Owned(string))
            }
        };

        let mut it = text
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
                    let c =
                        ast::utils::parse_char_escape(span, &mut it, ast::utils::WithBrace(true))?;
                    buf.push(c);
                }
                '}' => {
                    return Err(ParseError::new(span, ParseErrorKind::UnexpectedCloseBrace));
                }
                '{' => {
                    if !buf.is_empty() {
                        size_hint += buf.len();
                        components.push(TemplateComponent::String(buf.clone()));
                        buf.clear();
                    }

                    let span = ast::utils::template_expr(span, &mut it)?;
                    let source = &source.as_str()[..span.end];

                    let mut parser = Parser::new_with_start(source, span.start);
                    let expr = ast::Expr::parse(&mut parser)?;
                    components.push(TemplateComponent::Expr(Box::new(expr)));
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
/// parse_all::<ast::LitTemplate>("`hello world`").unwrap();
/// parse_all::<ast::LitTemplate>("`hello\\n world`").unwrap();
/// ```
impl Parse for LitTemplate {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        match token.kind {
            ast::Kind::LitTemplate(source) => Ok(LitTemplate { token, source }),
            _ => Err(ParseError::new(
                token,
                ParseErrorKind::ExpectedString { actual: token.kind },
            )),
        }
    }
}
