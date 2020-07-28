//! AST for the Rune language.

use crate::error::{ParseError, ResolveError, Result};
use crate::parser::Parser;
use crate::source::Source;
use crate::token::{self, Delimiter, Kind, Span, Token};
use crate::traits::{Parse, Peek, Resolve};
use std::borrow::Cow;

/// A parsed file.
pub struct File {
    /// All function declarations in the file.
    pub functions: Vec<FnDecl>,
}

/// Parse a file.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast, Resolve as _};
///
/// # fn main() -> anyhow::Result<()> {
/// let _ = parse_all::<ast::File>(r#"
/// fn foo() {
///     42
/// }
///
/// fn bar(a, b) {
///     a
/// }
/// "#)?;
/// # Ok(())
/// # }
/// ```
impl Parse for File {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let mut functions = Vec::new();

        while !parser.is_eof()? {
            functions.push(parser.parse::<FnDecl>()?);
        }

        Ok(Self { functions })
    }
}

/// A resolved number literal.
pub enum Number {
    /// A float literal number.
    Float(f64),
    /// An integer literal number.
    Integer(i64),
}

/// A number literal.
#[derive(Debug)]
pub struct ArrayLiteral {
    /// The open bracket.
    pub open: OpenBracket,
    /// Items in the array.
    pub items: Vec<Expr>,
    /// The close bracket.
    pub close: CloseBracket,
}

/// Parse an array literal.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast, Resolve as _};
///
/// # fn main() -> anyhow::Result<()> {
/// let _ = parse_all::<ast::ArrayLiteral>("[1, \"two\"]")?;
/// let _ = parse_all::<ast::ArrayLiteral>("[1, 2,]")?;
/// let _ = parse_all::<ast::ArrayLiteral>("[1, 2, foo()]")?;
/// # Ok(())
/// # }
/// ```
impl Parse for ArrayLiteral {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let open = parser.parse()?;

        let mut items = Vec::new();

        while !parser.peek::<CloseBracket>()? {
            items.push(parser.parse()?);

            if parser.peek::<Comma>()? {
                parser.parse::<Comma>()?;
            } else {
                break;
            }
        }

        let close = parser.parse()?;
        Ok(Self { open, items, close })
    }
}

/// A number literal.
#[derive(Debug)]
pub struct NumberLiteral {
    /// The kind of the number literal.
    number: token::NumberLiteral,
    /// The token corresponding to the literal.
    token: Token,
}

/// Parse a number literal.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast, Resolve as _};
///
/// # fn main() -> anyhow::Result<()> {
/// let _ = parse_all::<ast::NumberLiteral>("42")?;
/// # Ok(())
/// # }
/// ```
impl Parse for NumberLiteral {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        Ok(match token.kind {
            Kind::NumberLiteral { number } => NumberLiteral { number, token },
            _ => {
                return Err(ParseError::ExpectedNumberError {
                    actual: token.kind,
                    span: token.span,
                })
            }
        })
    }
}

impl<'a> Resolve<'a> for NumberLiteral {
    type Output = Number;

    fn resolve(self, source: Source<'a>) -> Result<Number, ResolveError> {
        let string = source.source(self.token.span)?;

        let number = match self.number {
            token::NumberLiteral::Binary => {
                i64::from_str_radix(&string[2..], 2).map_err(err_span(self.token.span))?
            }
            token::NumberLiteral::Octal => {
                i64::from_str_radix(&string[2..], 8).map_err(err_span(self.token.span))?
            }
            token::NumberLiteral::Hex => {
                i64::from_str_radix(&string[2..], 16).map_err(err_span(self.token.span))?
            }
            token::NumberLiteral::Decimal => {
                i64::from_str_radix(string, 10).map_err(err_span(self.token.span))?
            }
        };

        return Ok(Number::Integer(number));

        fn err_span<E>(span: Span) -> impl Fn(E) -> ResolveError {
            move |_| ResolveError::IllegalNumberLiteral { span }
        }
    }
}

/// A string literal.
#[derive(Debug)]
pub struct StringLiteral {
    /// The token corresponding to the literal.
    token: Token,
    /// If the string literal is escaped.
    escaped: bool,
}

impl StringLiteral {
    fn parse_escaped(self, source: &str) -> Result<String, ResolveError> {
        let mut buffer = String::with_capacity(source.len());
        let mut it = source.chars();

        while let Some(c) = it.next() {
            match (c, it.clone().next()) {
                ('\\', Some('n')) => {
                    buffer.push('\n');
                    it.next();
                }
                ('\\', Some('r')) => {
                    buffer.push('\r');
                    it.next();
                }
                ('\\', Some('"')) => {
                    buffer.push('"');
                    it.next();
                }
                ('\\', other) => {
                    return Err(ResolveError::BadStringEscapeSequence {
                        c: other.unwrap_or_default(),
                        span: self.token.span,
                    })
                }
                (c, _) => {
                    buffer.push(c);
                }
            }
        }

        Ok(buffer)
    }
}

impl<'a> Resolve<'a> for StringLiteral {
    type Output = Cow<'a, str>;

    fn resolve(self, source: Source<'a>) -> Result<Cow<'a, str>, ResolveError> {
        let string = source.source(self.token.span.narrow(1))?;

        Ok(if self.escaped {
            Cow::Owned(self.parse_escaped(string)?)
        } else {
            Cow::Borrowed(string)
        })
    }
}

/// Parse a string literal.
///
/// # Examples
///
/// ```rust
/// use rune::{ParseAll, parse_all, ast, Resolve as _};
///
/// # fn main() -> anyhow::Result<()> {
/// let ParseAll { source, item } = parse_all::<ast::StringLiteral>("\"hello world\"")?;
/// assert_eq!(item.resolve(source)?, "hello world");
///
/// let ParseAll { source, item } = parse_all::<ast::StringLiteral>("\"hello\\nworld\"")?;
/// assert_eq!(item.resolve(source)?, "hello\nworld");
/// # Ok(())
/// # }
/// ```
impl Parse for StringLiteral {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        match token.kind {
            Kind::StringLiteral { escaped } => Ok(StringLiteral { token, escaped }),
            _ => Err(ParseError::ExpectedStringError {
                actual: token.kind,
                span: token.span,
            }),
        }
    }
}

/// A simple operation.
#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    /// Addition.
    Add {
        /// Token associated with operator.
        token: Token,
    },
    /// Subtraction.
    Sub {
        /// Token associated with operator.
        token: Token,
    },
    /// Division.
    Div {
        /// Token associated with operator.
        token: Token,
    },
    /// Multiplication.
    Mul {
        /// Token associated with operator.
        token: Token,
    },
    /// Equality check.
    Eq {
        /// Token associated with operator.
        token: Token,
    },
    /// Greater-than check.
    Gt {
        /// Token associated with operator.
        token: Token,
    },
    /// Less-than check.
    Lt {
        /// Token associated with operator.
        token: Token,
    },
    /// Greater-than or equal check.
    Gte {
        /// Token associated with operator.
        token: Token,
    },
    /// Less-than or equal check.
    Lte {
        /// Token associated with operator.
        token: Token,
    },
}

impl BinOp {
    /// Get the precedence for the current operator.
    fn precedence(self) -> usize {
        match self {
            Self::Add { .. } | Self::Sub { .. } => 1,
            Self::Div { .. } | Self::Mul { .. } => 10,
            Self::Eq { .. } => 20,
            Self::Gt { .. } | Self::Lt { .. } => 20,
            Self::Gte { .. } | Self::Lte { .. } => 20,
        }
    }

    /// Convert from a token.
    fn from_token(token: Token) -> Option<BinOp> {
        Some(match token.kind {
            Kind::Plus => Self::Add { token },
            Kind::Minus => Self::Sub { token },
            Kind::Slash => Self::Div { token },
            Kind::Star => Self::Mul { token },
            Kind::EqEq => Self::Eq { token },
            Kind::Lt => Self::Lt { token },
            Kind::Gt => Self::Gt { token },
            Kind::Lte => Self::Lte { token },
            Kind::Gte => Self::Gte { token },
            _ => return None,
        })
    }
}

impl Parse for BinOp {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        Ok(match Self::from_token(token) {
            Some(bin_op) => bin_op,
            None => {
                return Err(ParseError::ExpectedOperator {
                    span: token.span,
                    actual: token.kind,
                })
            }
        })
    }
}

impl Peek for BinOp {
    fn peek(token: Option<Token>) -> bool {
        match token {
            Some(token) => match token.kind {
                Kind::Plus => true,
                Kind::Minus => true,
                Kind::Star => true,
                Kind::Slash => true,
                Kind::EqEq => true,
                Kind::Gt => true,
                Kind::Lt => true,
                Kind::Gte => true,
                Kind::Lte => true,
                _ => false,
            },
            None => false,
        }
    }
}

/// A binary expression.
#[derive(Debug)]
pub struct ExprBinary {
    /// The left-hand side of a binary operation.
    pub lhs: Box<Expr>,
    /// The operation to apply.
    pub op: BinOp,
    /// The right-hand side of a binary operation.
    pub rhs: Box<Expr>,
}

/// An else branch of an if expression.
#[derive(Debug)]
pub struct ExprIfElse {
    /// The `else` token.
    pub else_: ElseToken,
    /// The body of the else statement.
    pub else_branch: Box<Block>,
}

impl Parse for ExprIfElse {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(ExprIfElse {
            else_: parser.parse()?,
            else_branch: Box::new(parser.parse()?),
        })
    }
}

/// An if expression.
#[derive(Debug)]
pub struct ExprIf {
    /// The `if` token.
    pub if_: IfToken,
    /// The condition to the if statement.
    pub condition: Box<Expr>,
    /// The body of the if statement.
    pub then_branch: Box<Block>,
    /// The else part of the if expression.
    pub expr_if_else: Option<ExprIfElse>,
}

impl Parse for ExprIf {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let if_ = parser.parse()?;
        let condition = Box::new(parser.parse()?);
        let then_branch = Box::new(parser.parse()?);

        let expr_if_else = if parser.peek::<ElseToken>()? {
            Some(parser.parse()?)
        } else {
            None
        };

        Ok(ExprIf {
            if_,
            condition,
            then_branch,
            expr_if_else,
        })
    }
}

/// Argument to indicate if we are in an instance call.
struct SupportInstanceCall(bool);

/// A rune expression.
#[derive(Debug)]
pub enum Expr {
    /// An if expression.
    ExprIf(ExprIf),
    /// An empty expression.
    Ident(Ident),
    /// A function call,
    CallFn(CallFn),
    /// An instance function call,
    CallInstanceFn(CallInstanceFn),
    /// A literal array declaration.
    ArrayLiteral(ArrayLiteral),
    /// A literal number expression.
    NumberLiteral(NumberLiteral),
    /// A literal string expression.
    StringLiteral(StringLiteral),
    /// A grouped expression.
    ExprGroup(ExprGroup),
    /// A binary expression.
    ExprBinary(ExprBinary),
}

impl Expr {
    /// Default parse function.
    fn parse_default(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        Self::parse_primary(parser, SupportInstanceCall(true))
    }

    /// Special parse function to parse an expression inside of an instance call.
    fn parse_in_instance_call(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        Self::parse_primary(parser, SupportInstanceCall(false))
    }

    /// Parse a single expression value.
    fn parse_primary(
        parser: &mut Parser<'_>,
        instance_call: SupportInstanceCall,
    ) -> Result<Self, ParseError> {
        let token = parser.token_peek_eof()?;

        Ok(match token.kind {
            Kind::If => Self::ExprIf(parser.parse()?),
            Kind::Ident => match parser.token_peek2()?.map(|t| t.kind) {
                Some(Kind::Open {
                    delimiter: Delimiter::Parenthesis,
                }) => Self::CallFn(parser.parse()?),
                Some(Kind::Dot) if instance_call.0 => Self::CallInstanceFn(parser.parse()?),
                _ => Self::Ident(parser.parse()?),
            },
            Kind::NumberLiteral { .. } => Self::NumberLiteral(parser.parse()?),
            Kind::StringLiteral { .. } => Self::StringLiteral(parser.parse()?),
            Kind::Open {
                delimiter: Delimiter::Parenthesis,
            } => Self::ExprGroup(parser.parse()?),
            Kind::Open {
                delimiter: Delimiter::Bracket,
            } => Self::ArrayLiteral(parser.parse()?),
            _ => {
                return Err(ParseError::ExpectedExprError {
                    actual: token.kind,
                    span: token.span,
                })
            }
        })
    }

    /// Parse a binary expression.
    fn parse_expr_binary(
        parser: &mut Parser<'_>,
        mut lhs: Self,
        min_precedence: usize,
    ) -> Result<Self, ParseError> {
        let mut lookahead = parser.token_peek()?.and_then(BinOp::from_token);

        loop {
            let op = match lookahead {
                Some(op) if op.precedence() >= min_precedence => op,
                _ => break,
            };

            parser.token_next()?;
            let mut rhs = Self::parse_default(parser)?;

            lookahead = parser.token_peek()?.and_then(BinOp::from_token);

            loop {
                let lh = match lookahead {
                    Some(lh) if lh.precedence() > op.precedence() => lh,
                    _ => break,
                };

                rhs = Self::parse_expr_binary(parser, rhs, lh.precedence())?;
                lookahead = parser.token_peek()?.and_then(BinOp::from_token);
            }

            lhs = Expr::ExprBinary(ExprBinary {
                lhs: Box::new(lhs),
                op,
                rhs: Box::new(rhs),
            });
        }

        Ok(lhs)
    }
}

/// Parsing a block expression.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast, Resolve as _};
///
/// # fn main() {
/// parse_all::<ast::Expr>("foo.bar()").unwrap();
/// parse_all::<ast::Expr>("var()").unwrap();
/// parse_all::<ast::Expr>("var").unwrap();
/// parse_all::<ast::Expr>("42").unwrap();
/// parse_all::<ast::Expr>("1 + 2 / 3 - 4 * 1").unwrap();
/// # }
/// ```
impl Parse for Expr {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let lhs = Self::parse_default(parser)?;
        Self::parse_expr_binary(parser, lhs, 0)
    }
}

/// A function call `<name>(<args>)`.
#[derive(Debug)]
pub struct CallFn {
    /// The name of the function being called.
    pub name: Ident,
    /// The arguments of the function call.
    pub args: FunctionArgs<Expr>,
}

/// Parsing a function call.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast, Resolve as _};
///
/// # fn main() -> anyhow::Result<()> {
/// let _ = parse_all::<ast::CallFn>("foo()")?;
/// # Ok(())
/// # }
/// ```
impl Parse for CallFn {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        Ok(CallFn {
            name: parser.parse()?,
            args: parser.parse()?,
        })
    }
}

/// An instance function call `<instance>.<name>(<args>)`.
#[derive(Debug)]
pub struct CallInstanceFn {
    /// The instance being called.
    pub instance: Box<Expr>,
    /// The parsed dot separator.
    pub dot: Dot,
    /// The name of the function being called.
    pub call_fn: CallFn,
}

/// Parsing an instance function call.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast, Resolve as _};
///
/// # fn main() -> anyhow::Result<()> {
/// let _ = parse_all::<ast::CallInstanceFn>("foo.bar()")?;
/// assert!(parse_all::<ast::CallInstanceFn>("foo.bar.baz()").is_err());
/// # Ok(())
/// # }
/// ```
impl Parse for CallInstanceFn {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        Ok(CallInstanceFn {
            instance: Box::new(Expr::parse_in_instance_call(parser)?),
            dot: parser.parse()?,
            call_fn: parser.parse()?,
        })
    }
}

/// A rune block expression.
#[derive(Debug)]
pub enum BlockExpr {
    /// An expression.
    Expr(Expr),
    /// A let expression.
    Let(Let),
}

/// Parsing a block expression.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast, Resolve as _};
///
/// # fn main() -> anyhow::Result<()> {
/// let _ = parse_all::<ast::BlockExpr>("var")?;
/// let _ = parse_all::<ast::BlockExpr>("42")?;
/// let _ = parse_all::<ast::BlockExpr>("let var = 42")?;
/// let _ = parse_all::<ast::BlockExpr>("let var = \"foo bar\"")?;
/// # Ok(())
/// # }
/// ```
impl Parse for BlockExpr {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        // NB: peek token for efficiency reasons.
        let token = parser.token_peek_eof()?;

        Ok(match token.kind {
            Kind::Let => Self::Let(Let {
                let_: parser.parse()?,
                name: parser.parse()?,
                eq: parser.parse()?,
                expr: parser.parse()?,
            }),
            _ => Self::Expr(parser.parse()?),
        })
    }
}

/// A let expression `let <name> = <expr>;`
#[derive(Debug)]
pub struct Let {
    /// The `let` keyword.
    pub let_: LetToken,
    /// The name of the binding.
    pub name: Ident,
    /// The equality keyword.
    pub eq: Eq,
    /// The expression the binding is assigned to.
    pub expr: Expr,
}

/// A function.
#[derive(Debug)]
pub struct FnDecl {
    /// The `fn` token.
    pub fn_: FnToken,
    /// The name of the function.
    pub name: Ident,
    /// The arguments of the function.
    pub args: FunctionArgs<Ident>,
    /// The body of the function.
    pub body: Block,
}

/// Parse implementation for a function.
///
/// # Examples
///
/// ```rust
/// use rune::{ParseAll, parse_all, ast, Resolve as _};
///
/// # fn main() -> anyhow::Result<()> {
/// let ParseAll { item, .. } = parse_all::<ast::FnDecl>("fn hello() {}")?;
/// assert_eq!(item.args.items.len(), 0);
///
/// let ParseAll  { source, item } = parse_all::<ast::FnDecl>("fn hello(foo, bar) {}")?;
/// assert_eq!(item.args.items.len(), 2);
/// assert_eq!(item.name.resolve(source)?, "hello");
/// assert_eq!(item.args.items[0].resolve(source)?, "foo");
/// assert_eq!(item.args.items[1].resolve(source)?, "bar");
/// # Ok(())
/// # }
/// ```
impl Parse for FnDecl {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        Ok(Self {
            fn_: parser.parse()?,
            name: parser.parse()?,
            args: parser.parse()?,
            body: parser.parse()?,
        })
    }
}

/// A block of expressions.
#[derive(Debug)]
pub struct Block {
    /// Expressions in the block.
    pub exprs: Vec<BlockExpr>,
    /// The implicit return statement.
    pub implicit_return: Option<BlockExpr>,
}

/// Parse implementation for a block.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// # fn main() -> anyhow::Result<()> {
/// let block = parse_all::<ast::Block>("{}")?.item;
/// assert_eq!(block.exprs.len(), 0);
/// assert!(block.implicit_return.is_none());
///
/// let block = parse_all::<ast::Block>("{ foo }")?.item;
/// assert_eq!(block.exprs.len(), 0);
/// assert!(block.implicit_return.is_some());
///
/// let block = parse_all::<ast::Block>("{ foo; }")?.item;
/// assert_eq!(block.exprs.len(), 1);
/// assert!(block.implicit_return.is_none());
///
/// let block = parse_all::<ast::Block>(r#"
///     {
///         let foo = 42;
///         let bar = "string";
///         baz
///     }
/// "#)?.item;
/// assert_eq!(block.exprs.len(), 2);
/// assert!(block.implicit_return.is_some());
/// # Ok(())
/// # }
/// ```
impl Parse for Block {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let mut exprs = Vec::new();

        parser.parse::<OpenBrace>()?;

        let mut implicit_return = None;

        while !parser.peek::<CloseBrace>()? {
            let expr = parser.parse()?;

            if parser.peek::<SemiColon>()? {
                exprs.push(expr);
                parser.parse::<SemiColon>()?;
            } else {
                implicit_return = Some(expr);
                break;
            }
        }

        parser.parse::<CloseBrace>()?;

        Ok(Block {
            exprs,
            implicit_return,
        })
    }
}

/// Something parenthesized and comma separated `(<T,>*)`.
#[derive(Debug)]
pub struct FunctionArgs<T> {
    /// The open parenthesis.
    pub open: OpenParen,
    /// The parenthesized type.
    pub items: Vec<T>,
    /// The close parenthesis.
    pub close: CloseParen,
}

/// Parse function arguments.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast, Resolve as _};
///
/// # fn main() -> anyhow::Result<()> {
/// let _ = parse_all::<ast::FunctionArgs<ast::Expr>>("(1, \"two\")")?;
/// let _ = parse_all::<ast::FunctionArgs<ast::Expr>>("(1, 2,)")?;
/// let _ = parse_all::<ast::FunctionArgs<ast::Expr>>("(1, 2, foo())")?;
/// # Ok(())
/// # }
/// ```
impl<T> Parse for FunctionArgs<T>
where
    T: Parse,
{
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let open = parser.parse()?;

        let mut items = Vec::new();

        while !parser.peek::<CloseParen>()? {
            items.push(parser.parse()?);

            if parser.peek::<Comma>()? {
                parser.parse::<Comma>()?;
            } else {
                break;
            }
        }

        let close = parser.parse()?;

        Ok(Self { open, items, close })
    }
}

/// Something parenthesized and comma separated `(<T,>*)`.
#[derive(Debug)]
pub struct ExprGroup {
    /// The open parenthesis.
    pub open: OpenParen,
    /// The grouped expression.
    pub expr: Box<Expr>,
    /// The close parenthesis.
    pub close: CloseParen,
}

impl Parse for ExprGroup {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        Ok(Self {
            open: parser.parse()?,
            expr: Box::new(parser.parse()?),
            close: parser.parse()?,
        })
    }
}

macro_rules! decl_tokens {
    ($(($parser:ident, $($kind:tt)*),)*) => {
        $(
            /// Helper parser for a specifik token kind
            #[derive(Debug, Clone, Copy)]
            pub struct $parser {
                /// Associated token.
                pub token: Token,
            }

            impl Parse for $parser {
                fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
                    let token = parser.token_next()?;

                    match token.kind {
                        $($kind)* => Ok(Self {
                            token,
                        }),
                        _ => Err(ParseError::TokenMismatch {
                            expected: $($kind)*,
                            actual: token.kind,
                            span: token.span,
                        }),
                    }
                }
            }

            impl Peek for $parser {
                fn peek(token: Option<Token>) -> bool {
                    match token {
                        Some(token) => match token.kind {
                            $($kind)* => true,
                            _ => false,
                        }
                        _ => false,
                    }
                }
            }
        )*
    }
}

decl_tokens! {
    (FnToken, Kind::Fn),
    (IfToken, Kind::If),
    (ElseToken, Kind::Else),
    (LetToken, Kind::Let),
    (Ident, Kind::Ident),
    (OpenParen, Kind::Open { delimiter: Delimiter::Parenthesis }),
    (CloseParen, Kind::Close { delimiter: Delimiter::Parenthesis }),
    (OpenBrace, Kind::Open { delimiter: Delimiter::Brace }),
    (CloseBrace, Kind::Close { delimiter: Delimiter::Brace }),
    (OpenBracket, Kind::Open { delimiter: Delimiter::Bracket }),
    (CloseBracket, Kind::Close { delimiter: Delimiter::Bracket }),
    (Comma, Kind::Comma),
    (Dot, Kind::Dot),
    (SemiColon, Kind::SemiColon),
    (Eq, Kind::Eq),
}

impl<'a> Resolve<'a> for Ident {
    type Output = &'a str;

    fn resolve(self, source: Source<'a>) -> Result<&'a str, ResolveError> {
        source.source(self.token.span)
    }
}
