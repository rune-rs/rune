//! AST for the Rune language.

use crate::error::{ParseError, ResolveError, Result};
use crate::parser::Parser;
use crate::source::Source;
use crate::token::{self, Delimiter, Kind, Token};
use crate::traits::{Parse, Peek, Resolve};
use st::unit::Span;
use std::borrow::Cow;
use std::fmt;

#[derive(Debug, Clone, Copy)]
struct NoIndex(bool);

/// A parsed file.
pub struct File {
    /// Imports for the current file.
    pub imports: Vec<UseDecl>,
    /// All function declarations in the file.
    pub functions: Vec<FnDecl>,
}

/// Parse a file.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// # fn main() -> anyhow::Result<()> {
/// let _ = parse_all::<ast::File>(r#"
/// use foo;
///
/// fn foo() {
///     42
/// }
///
/// use bar;
///
/// fn bar(a, b) {
///     a
/// }
/// "#)?;
/// # Ok(())
/// # }
/// ```
///
/// # Realistic Example
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// # fn main() -> anyhow::Result<()> {
/// let _ = parse_all::<ast::File>(r#"
/// use http;
///
/// fn main() {
///     let client = http::client();
///     let response = client.get("https://google.com");
///     let text = response.text();
/// }
/// "#)?;
/// # Ok(())
/// # }
/// ```
impl Parse for File {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let mut imports = Vec::new();
        let mut functions = Vec::new();

        while !parser.is_eof()? {
            match parser.token_peek()?.map(|t| t.kind) {
                Some(Kind::Use) => {
                    imports.push(parser.parse()?);
                }
                _ => {
                    functions.push(parser.parse()?);
                }
            }
        }

        Ok(Self { imports, functions })
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
#[derive(Debug, Clone)]
pub struct ArrayLiteral {
    /// The open bracket.
    pub open: OpenBracket,
    /// Items in the array.
    pub items: Vec<Expr>,
    /// The close bracket.
    pub close: CloseBracket,
}

impl ArrayLiteral {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.open.span().join(self.close.span())
    }

    /// Test if the entire expression is literal.
    pub fn is_all_literal(&self) -> bool {
        self.items.iter().all(|e| e.is_all_literal())
    }
}

/// Parse an array literal.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast, Resolve as _};
///
/// # fn main() -> anyhow::Result<()> {
/// parse_all::<ast::ArrayLiteral>("[1, \"two\"]").unwrap();
/// parse_all::<ast::ArrayLiteral>("[1, 2,]").unwrap();
/// parse_all::<ast::ArrayLiteral>("[1, 2, foo()]").unwrap();
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
#[derive(Debug, Clone)]
pub struct ObjectLiteral {
    /// The open bracket.
    pub open: StartObject,
    /// Items in the object declaration.
    pub items: Vec<(StringLiteral, Colon, Expr)>,
    /// The close bracket.
    pub close: CloseBrace,
}

impl ObjectLiteral {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.open.span().join(self.close.span())
    }

    /// Test if the entire expression is literal.
    pub fn is_all_literal(&self) -> bool {
        self.items.iter().all(|e| e.2.is_all_literal())
    }
}

/// Parse an object literal.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// # fn main() -> anyhow::Result<()> {
/// parse_all::<ast::ObjectLiteral>("#{\"foo\": 42}")?;
/// parse_all::<ast::ObjectLiteral>("#{\"foo\": 42,}")?;
/// # Ok(())
/// # }
/// ```
impl Parse for ObjectLiteral {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let open = parser.parse()?;

        let mut items = Vec::new();

        while !parser.peek::<CloseBrace>()? {
            let key = parser.parse()?;
            let colon = parser.parse()?;
            let expr = parser.parse()?;
            items.push((key, colon, expr));

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
#[derive(Debug, Clone)]
pub struct CharLiteral {
    /// The token corresponding to the literal.
    pub token: Token,
}

impl CharLiteral {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.token.span
    }
}

/// Parse a number literal.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// # fn main() -> anyhow::Result<()> {
/// parse_all::<ast::CharLiteral>("'a'")?;
/// parse_all::<ast::CharLiteral>("'\\0'")?;
/// parse_all::<ast::CharLiteral>("'\\n'")?;
/// parse_all::<ast::CharLiteral>("'\\r'")?;
/// parse_all::<ast::CharLiteral>("'\\''")?;
/// # Ok(())
/// # }
/// ```
impl Parse for CharLiteral {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        Ok(match token.kind {
            Kind::CharLiteral => CharLiteral { token },
            _ => {
                return Err(ParseError::ExpectedCharError {
                    actual: token.kind,
                    span: token.span,
                })
            }
        })
    }
}

impl CharLiteral {
    fn parse_escaped(
        &self,
        span: Span,
        mut it: impl Iterator<Item = char>,
    ) -> Result<char, ResolveError> {
        let mut next = || {
            it.next()
                .ok_or_else(|| ResolveError::BadCharLiteral { span })
        };

        Ok(match next()? {
            '\'' => '\'',
            '0' => '\0',
            'n' => '\n',
            'r' => '\r',
            // TODO: parse unicode literal.
            _ => return Err(ResolveError::BadCharLiteral { span }),
        })
    }
}

impl<'a> Resolve<'a> for CharLiteral {
    type Output = char;

    fn resolve(&self, source: Source<'a>) -> Result<char, ResolveError> {
        let span = self.token.span;
        let string = source.source(span.narrow(1))?;

        let mut it = string.chars();

        let c = loop {
            let c = match it.next() {
                Some(c) => c,
                None => {
                    return Err(ResolveError::BadCharLiteral { span });
                }
            };

            break match c {
                '\\' => self.parse_escaped(span, &mut it)?,
                o => o,
            };
        };

        // Too many characters in literal.
        if it.next().is_some() {
            return Err(ResolveError::BadCharLiteral { span });
        }

        Ok(c)
    }
}

/// A number literal.
#[derive(Debug, Clone)]
pub struct NumberLiteral {
    /// Indicates if the number is fractional.
    is_fractional: bool,
    /// The kind of the number literal.
    number: token::NumberLiteral,
    /// The token corresponding to the literal.
    token: Token,
}

impl NumberLiteral {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.token.span
    }
}

/// Parse a number literal.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// # fn main() -> anyhow::Result<()> {
/// parse_all::<ast::NumberLiteral>("42")?;
/// # Ok(())
/// # }
/// ```
impl Parse for NumberLiteral {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        Ok(match token.kind {
            Kind::NumberLiteral {
                is_fractional,
                number,
            } => NumberLiteral {
                is_fractional,
                number,
                token,
            },
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

    fn resolve(&self, source: Source<'a>) -> Result<Number, ResolveError> {
        use std::str::FromStr as _;

        let mut string = source.source(self.token.span)?;
        let mut is_negative = false;

        if string.starts_with('-') {
            string = &string[..];
            is_negative = true;
        }

        if self.is_fractional {
            let mut number = f64::from_str(string).map_err(err_span(self.token.span))?;

            if is_negative {
                number = -number;
            }

            return Ok(Number::Float(number));
        }

        let (s, radix) = match self.number {
            token::NumberLiteral::Binary => (2, 2),
            token::NumberLiteral::Octal => (2, 8),
            token::NumberLiteral::Hex => (2, 16),
            token::NumberLiteral::Decimal => (0, 10),
        };

        let mut number =
            i64::from_str_radix(&string[s..], radix).map_err(err_span(self.token.span))?;

        if is_negative {
            number = -number;
        }

        return Ok(Number::Integer(number));

        fn err_span<E>(span: Span) -> impl Fn(E) -> ResolveError {
            move |_| ResolveError::IllegalNumberLiteral { span }
        }
    }
}

/// A string literal.
#[derive(Debug, Clone)]
pub struct StringLiteral {
    /// The token corresponding to the literal.
    token: Token,
    /// If the string literal is escaped.
    escaped: bool,
}

impl StringLiteral {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.token.span
    }
}

impl StringLiteral {
    fn parse_escaped(&self, source: &str) -> Result<String, ResolveError> {
        let mut buffer = String::with_capacity(source.len());
        let mut it = source.chars();

        while let Some(c) = it.next() {
            match (c, it.clone().next()) {
                ('\\', Some('0')) => {
                    buffer.push('\0');
                    it.next();
                }
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

    fn resolve(&self, source: Source<'a>) -> Result<Cow<'a, str>, ResolveError> {
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

/// A unary operation.
#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    /// Not `!<thing>`.
    Not {
        /// Token associated with operator.
        token: Token,
    },
    /// Reference `&<thing>`.
    Ref {
        /// Token associated with operator.
        token: Token,
    },
    /// Dereference `*<thing>`.
    Deref {
        /// Token associated with operator.
        token: Token,
    },
}

impl UnaryOp {
    /// Access the span of the unary operator.
    pub fn span(&self) -> Span {
        match self {
            Self::Not { token } => token.span,
            Self::Ref { token } => token.span,
            Self::Deref { token } => token.span,
        }
    }
}

impl Parse for UnaryOp {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        Ok(match token.kind {
            Kind::Not => Self::Not { token },
            Kind::Ampersand => Self::Ref { token },
            Kind::Star => Self::Deref { token },
            actual => {
                return Err(ParseError::ExpectedUnaryOperator {
                    span: token.span,
                    actual,
                })
            }
        })
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Not { .. } => write!(fmt, "!")?,
            Self::Ref { .. } => write!(fmt, "&")?,
            Self::Deref { .. } => write!(fmt, "*")?,
        }

        Ok(())
    }
}

/// A binary operation.
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
    /// Inequality check.
    Neq {
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
    /// The dot operator.
    Dot {
        /// Token associated with operator.
        token: Token,
    },
    /// The instanceof test.
    Is {
        /// Token associated with operator.
        token: Token,
    },
    /// Assign operation.
    Assign {
        /// Token associated with operator.
        token: Token,
    },
}

impl BinOp {
    /// Get the precedence for the current operator.
    fn precedence(self) -> usize {
        match self {
            // `is` has lowest precedence.
            Self::Assign { .. } => 0,
            Self::Is { .. } => 5,
            Self::Add { .. } | Self::Sub { .. } => 10,
            Self::Div { .. } | Self::Mul { .. } => 20,
            Self::Eq { .. } | Self::Neq { .. } => 30,
            Self::Gt { .. } => 40,
            Self::Lt { .. } => 41,
            Self::Gte { .. } => 42,
            Self::Lte { .. } => 43,
            Self::Dot { .. } => 60,
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
            Kind::Neq => Self::Neq { token },
            Kind::Lt => Self::Lt { token },
            Kind::Gt => Self::Gt { token },
            Kind::Lte => Self::Lte { token },
            Kind::Gte => Self::Gte { token },
            Kind::Dot => Self::Dot { token },
            Kind::Is => Self::Is { token },
            Kind::Eq => Self::Assign { token },
            _ => return None,
        })
    }
}

impl fmt::Display for BinOp {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            BinOp::Add { .. } => {
                write!(fmt, "+")?;
            }
            BinOp::Sub { .. } => {
                write!(fmt, "-")?;
            }
            BinOp::Div { .. } => {
                write!(fmt, "/")?;
            }
            BinOp::Mul { .. } => {
                write!(fmt, "*")?;
            }
            BinOp::Eq { .. } => {
                write!(fmt, "==")?;
            }
            BinOp::Neq { .. } => {
                write!(fmt, "!=")?;
            }
            BinOp::Gt { .. } => {
                write!(fmt, ">")?;
            }
            BinOp::Lt { .. } => {
                write!(fmt, "<")?;
            }
            BinOp::Gte { .. } => {
                write!(fmt, ">=")?;
            }
            BinOp::Lte { .. } => {
                write!(fmt, "<=")?;
            }
            BinOp::Dot { .. } => {
                write!(fmt, ".")?;
            }
            BinOp::Is { .. } => {
                write!(fmt, "is")?;
            }
            BinOp::Assign { .. } => {
                write!(fmt, "=")?;
            }
        }

        Ok(())
    }
}

impl Parse for BinOp {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        Ok(match Self::from_token(token) {
            Some(bin_op) => bin_op,
            None => {
                return Err(ParseError::ExpectedOperatorError {
                    span: token.span,
                    actual: token.kind,
                })
            }
        })
    }
}

impl Peek for BinOp {
    fn peek(p1: Option<Token>, _: Option<Token>) -> bool {
        match p1 {
            Some(p1) => match p1.kind {
                Kind::Plus => true,
                Kind::Minus => true,
                Kind::Star => true,
                Kind::Slash => true,
                Kind::EqEq => true,
                Kind::Neq => true,
                Kind::Gt => true,
                Kind::Lt => true,
                Kind::Gte => true,
                Kind::Lte => true,
                Kind::Dot => true,
                Kind::Is => true,
                _ => false,
            },
            None => false,
        }
    }
}

/// A binary expression.
#[derive(Debug, Clone)]
pub struct ExprBinary {
    /// The left-hand side of a binary operation.
    pub lhs: Box<Expr>,
    /// The operation to apply.
    pub op: BinOp,
    /// The right-hand side of a binary operation.
    pub rhs: Box<Expr>,
}

impl ExprBinary {
    /// If the expression is empty.
    pub fn produces_nothing(&self) -> bool {
        match self.op {
            // Assignments do not produce a value.
            BinOp::Assign { .. } => false,
            _ => true,
        }
    }

    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.lhs.span().join(self.rhs.span())
    }
}

/// A unary expression.
#[derive(Debug, Clone)]
pub struct ExprUnary {
    /// The operation to apply.
    pub op: UnaryOp,
    /// The expression of the operation.
    pub expr: Box<Expr>,
}

impl ExprUnary {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.op.span().join(self.expr.span())
    }
}

impl Parse for ExprUnary {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(Self {
            op: parser.parse()?,
            expr: Box::new(Expr::parse_primary(parser, NoIndex(false))?),
        })
    }
}

/// An else branch of an if expression.
#[derive(Debug, Clone)]
pub struct ExprElseIf {
    /// The `else` token.
    pub else_: ElseToken,
    /// The `if` token.
    pub if_: IfToken,
    /// The condition for the branch.
    pub condition: Box<Expr>,
    /// The body of the else statement.
    pub block: Box<Block>,
}

impl ExprElseIf {
    /// Access the span for the expression.
    pub fn span(&self) -> Span {
        self.else_.span().join(self.block.span())
    }
}

impl Parse for ExprElseIf {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(Self {
            else_: parser.parse()?,
            if_: parser.parse()?,
            condition: Box::new(parser.parse()?),
            block: Box::new(parser.parse()?),
        })
    }
}

/// An else branch of an if expression.
#[derive(Debug, Clone)]
pub struct ExprElse {
    /// The `else` token.
    pub else_: ElseToken,
    /// The body of the else statement.
    pub block: Box<Block>,
}

impl ExprElse {
    /// Access the span for the expression.
    pub fn span(&self) -> Span {
        self.else_.span().join(self.block.span())
    }
}

impl Parse for ExprElse {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(ExprElse {
            else_: parser.parse()?,
            block: Box::new(parser.parse()?),
        })
    }
}

/// An if expression.
#[derive(Debug, Clone)]
pub struct ExprIf {
    /// The `if` token.
    pub if_: IfToken,
    /// The condition to the if statement.
    pub condition: Box<Expr>,
    /// The body of the if statement.
    pub block: Box<Block>,
    /// Else if branches.
    pub expr_else_ifs: Vec<ExprElseIf>,
    /// The else part of the if expression.
    pub expr_else: Option<ExprElse>,
}

impl ExprIf {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        if let Some(else_) = &self.expr_else {
            self.if_.token.span.join(else_.block.span())
        } else if let Some(else_if) = self.expr_else_ifs.last() {
            self.if_.token.span.join(else_if.block.span())
        } else {
            self.if_.token.span.join(self.block.span())
        }
    }

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
/// use rune::{ParseAll, parse_all, ast};
///
/// # fn main() -> anyhow::Result<()> {
/// parse_all::<ast::ExprIf>("if 0 {  }")?;
/// parse_all::<ast::ExprIf>("if 0 {  } else {  }")?;
/// parse_all::<ast::ExprIf>("if 0 {  } else if 0 {  } else {  }")?;
/// # Ok(())
/// # }
/// ```
impl Parse for ExprIf {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let if_ = parser.parse()?;
        let condition = Box::new(parser.parse()?);
        let block = Box::new(parser.parse()?);
        let mut expr_else_ifs = Vec::new();
        let mut expr_else = None;

        while parser.peek::<ElseToken>()? {
            if parser.peek2::<IfToken>()? {
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

/// A rune expression.
#[derive(Debug, Clone)]
pub enum Expr {
    /// A while loop.
    While(While),
    /// An unconditional loop.
    Loop(Loop),
    /// An for loop.
    For(For),
    /// A let expression.
    Let(Let),
    /// An index set operation.
    IndexSet(IndexSet),
    /// An if expression.
    ExprIf(ExprIf),
    /// An empty expression.
    Ident(Ident),
    /// An path expression.
    Path(Path),
    /// A function call,
    CallFn(CallFn),
    /// An instance function call,
    CallInstanceFn(CallInstanceFn),
    /// A boolean literal.
    BoolLiteral(BoolLiteral),
    /// A char literal.
    CharLiteral(CharLiteral),
    /// A literal number expression.
    NumberLiteral(NumberLiteral),
    /// A literal string expression.
    StringLiteral(StringLiteral),
    /// A literal array declaration.
    ArrayLiteral(ArrayLiteral),
    /// A literal object declaration.
    ObjectLiteral(ObjectLiteral),
    /// A grouped expression.
    ExprGroup(ExprGroup),
    /// A binary expression.
    ExprBinary(ExprBinary),
    /// A unary expression.
    ExprUnary(ExprUnary),
    /// An index set operation.
    IndexGet(IndexGet),
    /// A unit expression.
    UnitLiteral(UnitLiteral),
    /// A break expression.
    Break(Break),
    /// A block as an expression.
    Block(Block),
    /// A return statement.
    Return(Return),
}

impl Expr {
    /// Test if the expression implicitly evaluates to nothing.
    pub fn produces_nothing(&self) -> bool {
        match self {
            Self::While(..) => true,
            Self::Loop(..) => true,
            Self::For(..) => true,
            Self::Let(..) => true,
            Self::IndexSet(..) => true,
            Self::ExprIf(expr_if) => expr_if.produces_nothing(),
            Self::ExprGroup(expr_group) => expr_group.produces_nothing(),
            Self::Break(..) => true,
            Self::ExprBinary(expr) => expr.produces_nothing(),
            Self::Block(b) => b.produces_nothing(),
            Self::Return(..) => true,
            _ => false,
        }
    }

    /// Get the span of the expression.
    pub fn span(&self) -> Span {
        match self {
            Self::While(expr) => expr.span(),
            Self::Loop(expr) => expr.span(),
            Self::For(expr) => expr.span(),
            Self::Let(expr) => expr.span(),
            Self::IndexSet(expr) => expr.span(),
            Self::ExprIf(expr) => expr.span(),
            Self::Ident(expr) => expr.span(),
            Self::Path(path) => path.span(),
            Self::CallFn(expr) => expr.span(),
            Self::CallInstanceFn(expr) => expr.span(),
            Self::ArrayLiteral(expr) => expr.span(),
            Self::ObjectLiteral(expr) => expr.span(),
            Self::NumberLiteral(expr) => expr.span(),
            Self::CharLiteral(expr) => expr.span(),
            Self::StringLiteral(expr) => expr.span(),
            Self::ExprGroup(expr) => expr.span(),
            Self::ExprUnary(expr) => expr.span(),
            Self::ExprBinary(expr) => expr.span(),
            Self::IndexGet(expr) => expr.span(),
            Self::UnitLiteral(unit) => unit.span(),
            Self::BoolLiteral(b) => b.span(),
            Self::Break(b) => b.span(),
            Self::Block(b) => b.span(),
            Self::Return(ret) => ret.span(),
        }
    }

    /// Test if the entire expression is literal.
    pub fn is_all_literal(&self) -> bool {
        match self {
            Expr::UnitLiteral(..) => true,
            Expr::BoolLiteral(..) => true,
            Expr::CharLiteral(..) => true,
            Expr::NumberLiteral(..) => true,
            Expr::StringLiteral(..) => true,
            Expr::ArrayLiteral(array) => array.is_all_literal(),
            Expr::ObjectLiteral(object) => object.is_all_literal(),
            Expr::Block(b) => b.is_all_literal(),
            _ => false,
        }
    }

    /// Parse expressions that start with an identifier.
    pub fn parse_ident_start(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let path = parser.parse::<Path>()?;

        if !parser.peek::<OpenParen>()? {
            if path.rest.is_empty() {
                return Ok(Self::Ident(path.first));
            }

            return Ok(Self::Path(path));
        }

        Ok(Self::CallFn(CallFn {
            name: path,
            args: parser.parse()?,
        }))
    }

    /// Parse indexing operation.
    pub fn parse_indexing_op(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let index_get = IndexGet {
            target: Box::new(Self::parse_primary(parser, NoIndex(true))?),
            open: parser.parse()?,
            index: Box::new(parser.parse()?),
            close: parser.parse()?,
        };

        Ok(if parser.peek::<Eq>()? {
            Self::IndexSet(IndexSet {
                target: index_get.target,
                open: index_get.open,
                index: index_get.index,
                close: index_get.close,
                eq: parser.parse()?,
                value: Box::new(parser.parse()?),
            })
        } else {
            Self::IndexGet(index_get)
        })
    }

    /// Parse a single expression value.
    fn parse_primary(parser: &mut Parser<'_>, no_index: NoIndex) -> Result<Self, ParseError> {
        let token = parser.token_peek_eof()?;

        let expr = match token.kind {
            Kind::StartObject => Self::ObjectLiteral(parser.parse()?),
            Kind::Not | Kind::Ampersand | Kind::Star => Self::ExprUnary(parser.parse()?),
            Kind::While => Self::While(parser.parse()?),
            Kind::Loop => Self::Loop(parser.parse()?),
            Kind::For => Self::For(parser.parse()?),
            Kind::Let => Self::Let(parser.parse()?),
            Kind::If => Self::ExprIf(parser.parse()?),
            Kind::NumberLiteral { .. } => Self::NumberLiteral(parser.parse()?),
            Kind::CharLiteral { .. } => Self::CharLiteral(parser.parse()?),
            Kind::StringLiteral { .. } => Self::StringLiteral(parser.parse()?),
            Kind::Open {
                delimiter: Delimiter::Parenthesis,
            } => {
                if parser.peek::<UnitLiteral>()? {
                    Self::UnitLiteral(parser.parse()?)
                } else {
                    Self::ExprGroup(parser.parse()?)
                }
            }
            Kind::Open {
                delimiter: Delimiter::Bracket,
            } => Self::ArrayLiteral(parser.parse()?),
            Kind::Open {
                delimiter: Delimiter::Brace,
            } => Self::Block(parser.parse()?),
            Kind::True | Kind::False => Self::BoolLiteral(parser.parse()?),
            Kind::Ident => match parser.token_peek2()?.map(|t| t.kind) {
                Some(kind) => match kind {
                    Kind::Open {
                        delimiter: Delimiter::Bracket,
                    } if !no_index.0 => Self::parse_indexing_op(parser)?,
                    _ => Self::parse_ident_start(parser)?,
                },
                None => Self::parse_ident_start(parser)?,
            },
            Kind::Break => Self::Break(parser.parse()?),
            Kind::Return => Self::Return(parser.parse()?),
            _ => {
                return Err(ParseError::ExpectedExprError {
                    actual: token.kind,
                    span: token.span,
                })
            }
        };

        Ok(Self::parse_expr_chain(parser, expr, no_index)?)
    }

    /// Parse an expression chain.
    fn parse_expr_chain(
        parser: &mut Parser<'_>,
        mut expr: Self,
        no_index: NoIndex,
    ) -> Result<Self, ParseError> {
        loop {
            let token = match parser.token_peek()? {
                Some(token) => token,
                None => break,
            };

            match token.kind {
                Kind::Open {
                    delimiter: Delimiter::Bracket,
                } if !no_index.0 => {
                    expr = Expr::IndexGet(IndexGet {
                        target: Box::new(expr),
                        open: parser.parse()?,
                        index: Box::new(parser.parse()?),
                        close: parser.parse()?,
                    });
                }
                _ => break,
            }
        }

        Ok(expr)
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
            let mut rhs = Self::parse_primary(parser, NoIndex(false))?;

            lookahead = parser.token_peek()?.and_then(BinOp::from_token);

            loop {
                let lh = match lookahead {
                    Some(lh) if lh.precedence() > op.precedence() => lh,
                    _ => break,
                };

                rhs = Self::parse_expr_binary(parser, rhs, lh.precedence())?;
                lookahead = parser.token_peek()?.and_then(BinOp::from_token);
            }

            lhs = match (op, rhs) {
                (BinOp::Dot { token }, Expr::CallFn(call_fn)) => {
                    let name = call_fn.name.into_instance_call_ident()?;

                    Expr::CallInstanceFn(CallInstanceFn {
                        instance: Box::new(lhs),
                        dot: Dot { token },
                        name,
                        args: call_fn.args,
                    })
                }
                (op, rhs) => Expr::ExprBinary(ExprBinary {
                    lhs: Box::new(lhs),
                    op,
                    rhs: Box::new(rhs),
                }),
            }
        }

        Ok(lhs)
    }
}

/// The unit literal `()`.
#[derive(Debug, Clone)]
pub struct UnitLiteral {
    /// The open parenthesis.
    pub open: OpenParen,
    /// The close parenthesis.
    pub close: CloseParen,
}

impl UnitLiteral {
    /// Get the span of this unit literal.
    pub fn span(&self) -> Span {
        self.open.span().join(self.close.span())
    }
}

/// Parsing a unit literal
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// # fn main() {
/// parse_all::<ast::UnitLiteral>("()").unwrap();
/// # }
/// ```
impl Parse for UnitLiteral {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(Self {
            open: parser.parse()?,
            close: parser.parse()?,
        })
    }
}

impl Peek for UnitLiteral {
    fn peek(p1: Option<Token>, p2: Option<Token>) -> bool {
        let (p1, p2) = match (p1, p2) {
            (Some(p1), Some(p2)) => (p1, p2),
            _ => return false,
        };

        match (p1.kind, p2.kind) {
            (
                Kind::Open {
                    delimiter: Delimiter::Parenthesis,
                },
                Kind::Close {
                    delimiter: Delimiter::Parenthesis,
                },
            ) => true,
            _ => false,
        }
    }
}

/// The unit literal `()`.
#[derive(Debug, Clone)]
pub struct BoolLiteral {
    /// The value of the literal.
    pub value: bool,
    /// The token of the literal.
    pub token: Token,
}

impl BoolLiteral {
    /// Get the span of this unit literal.
    pub fn span(&self) -> Span {
        self.token.span
    }
}

/// Parsing a unit literal
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// # fn main() {
/// parse_all::<ast::BoolLiteral>("true").unwrap();
/// parse_all::<ast::BoolLiteral>("false").unwrap();
/// # }
/// ```
impl Parse for BoolLiteral {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        let token = parser.token_next()?;

        let value = match token.kind {
            Kind::True => true,
            Kind::False => false,
            _ => {
                return Err(ParseError::ExpectedBoolError {
                    span: token.span,
                    actual: token.kind,
                })
            }
        };

        Ok(Self { value, token })
    }
}

impl Peek for BoolLiteral {
    fn peek(p1: Option<Token>, _: Option<Token>) -> bool {
        let p1 = match p1 {
            Some(p1) => p1,
            None => return false,
        };

        match p1.kind {
            Kind::True => true,
            Kind::False => true,
            _ => false,
        }
    }
}

/// Parsing a block expression.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// # fn main() {
/// parse_all::<ast::Expr>("foo[\"foo\"]").unwrap();
/// parse_all::<ast::Expr>("foo.bar()").unwrap();
/// parse_all::<ast::Expr>("var()").unwrap();
/// parse_all::<ast::Expr>("var").unwrap();
/// parse_all::<ast::Expr>("42").unwrap();
/// parse_all::<ast::Expr>("1 + 2 / 3 - 4 * 1").unwrap();
/// parse_all::<ast::Expr>("foo[\"bar\"]").unwrap();
/// parse_all::<ast::Expr>("let var = 42").unwrap();
/// parse_all::<ast::Expr>("let var = \"foo bar\"").unwrap();
/// parse_all::<ast::Expr>("var[\"foo\"] = \"bar\"").unwrap();
/// parse_all::<ast::Expr>("let var = objects[\"foo\"] + 1").unwrap();
/// parse_all::<ast::Expr>("var = 42").unwrap();
///
/// let expr = parse_all::<ast::Expr>(r#"
///     if 1 { } else { if 2 { } else { } }
/// "#).unwrap();
///
/// if let ast::Expr::ExprIf(..) = expr.item {
/// } else {
///     panic!("not an if statement");
/// }
///
/// // Chained function calls.
/// parse_all::<ast::Expr>("foo.bar.baz()").unwrap();
/// parse_all::<ast::Expr>("foo[0][1][2]").unwrap();
/// parse_all::<ast::Expr>("foo.bar()[0].baz()[1]").unwrap();
///
/// parse_all::<ast::Expr>("42 is int::int").unwrap();
/// # }
/// ```
impl Parse for Expr {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let lhs = Self::parse_primary(parser, NoIndex(false))?;
        Ok(Self::parse_expr_binary(parser, lhs, 0)?)
    }
}

impl Peek for Expr {
    fn peek(t1: Option<Token>, _: Option<Token>) -> bool {
        let t1 = match t1 {
            Some(t1) => t1,
            None => return false,
        };

        match t1.kind {
            Kind::StartObject => true,
            Kind::Not | Kind::Ampersand | Kind::Star => true,
            Kind::While => true,
            Kind::Loop => true,
            Kind::For => true,
            Kind::Let => true,
            Kind::If => true,
            Kind::NumberLiteral { .. } => true,
            Kind::CharLiteral { .. } => true,
            Kind::StringLiteral { .. } => true,
            Kind::Open {
                delimiter: Delimiter::Parenthesis,
            } => true,
            Kind::Open {
                delimiter: Delimiter::Bracket,
            } => true,
            Kind::Open {
                delimiter: Delimiter::Brace,
            } => true,
            Kind::True | Kind::False => true,
            Kind::Ident => true,
            Kind::Break => true,
            Kind::Return => true,
            _ => false,
        }
    }
}

/// A function call `<name>(<args>)`.
#[derive(Debug, Clone)]
pub struct CallFn {
    /// The name of the function being called.
    pub name: Path,
    /// The arguments of the function call.
    pub args: FunctionArgs<Expr>,
}

impl CallFn {
    /// Access the span of expression.
    pub fn span(&self) -> Span {
        self.name.span().join(self.args.span())
    }
}

/// An instance function call `<instance>.<name>(<args>)`.
#[derive(Debug, Clone)]
pub struct CallInstanceFn {
    /// The instance being called.
    pub instance: Box<Expr>,
    /// The parsed dot separator.
    pub dot: Dot,
    /// The name of the function being called.
    pub name: Ident,
    /// The arguments of the function call.
    pub args: FunctionArgs<Expr>,
}

impl CallInstanceFn {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.instance.span().join(self.args.span())
    }
}

/// A let expression `let <name> = <expr>;`
#[derive(Debug, Clone)]
pub struct Let {
    /// The `let` keyword.
    pub let_: LetToken,
    /// The name of the binding.
    pub name: Ident,
    /// The equality keyword.
    pub eq: Eq,
    /// The expression the binding is assigned to.
    pub expr: Box<Expr>,
}

impl Let {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.let_.token.span.join(self.expr.span())
    }
}

impl Parse for Let {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(Self {
            let_: parser.parse()?,
            name: parser.parse()?,
            eq: parser.parse()?,
            expr: Box::new(parser.parse()?),
        })
    }
}

/// A let expression `let <name> = <expr>;`
#[derive(Debug, Clone)]
pub struct While {
    /// The `while` keyword.
    pub while_: WhileToken,
    /// The name of the binding.
    pub condition: Box<Expr>,
    /// The body of the while loop.
    pub body: Box<Block>,
}

impl While {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.while_.token.span.join(self.body.span())
    }
}

impl Parse for While {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        Ok(While {
            while_: parser.parse()?,
            condition: Box::new(parser.parse()?),
            body: Box::new(parser.parse()?),
        })
    }
}

/// A let expression `let <name> = <expr>;`
#[derive(Debug, Clone)]
pub struct Loop {
    /// The `loop` keyword.
    pub loop_: LoopToken,
    /// The body of the loop.
    pub body: Box<Block>,
}

impl Loop {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.loop_.token.span.join(self.body.span())
    }
}

impl Parse for Loop {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        Ok(Loop {
            loop_: parser.parse()?,
            body: Box::new(parser.parse()?),
        })
    }
}

/// A let expression `let <name> = <expr>;`
#[derive(Debug, Clone)]
pub struct For {
    /// The `for` keyword.
    pub for_: ForToken,
    /// The variable binding.
    /// TODO: should be a pattern when that is supported.
    pub var: Ident,
    /// The `in` keyword.
    pub in_: InToken,
    /// Expression producing the iterator.
    pub iter: Box<Expr>,
    /// The body of the loop.
    pub body: Box<Block>,
}

impl For {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.for_.token.span.join(self.body.span())
    }
}

impl Parse for For {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        Ok(Self {
            for_: parser.parse()?,
            var: parser.parse()?,
            in_: parser.parse()?,
            iter: Box::new(parser.parse()?),
            body: Box::new(parser.parse()?),
        })
    }
}

/// An index set operation `<target>[<index>] = <value>`.
#[derive(Debug, Clone)]
pub struct IndexSet {
    /// The target of the index set.
    pub target: Box<Expr>,
    /// The opening bracket.
    pub open: OpenBracket,
    /// The indexing expression.
    pub index: Box<Expr>,
    /// The closening bracket.
    pub close: CloseBracket,
    /// The equals sign.
    pub eq: Eq,
    /// The value expression we are assigning.
    pub value: Box<Expr>,
}

impl IndexSet {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.target.span().join(self.value.span())
    }
}

/// An index get operation `<target>[<index>]`.
#[derive(Debug, Clone)]
pub struct IndexGet {
    /// The target of the index set.
    pub target: Box<Expr>,
    /// The opening bracket.
    pub open: OpenBracket,
    /// The indexing expression.
    pub index: Box<Expr>,
    /// The closening bracket.
    pub close: CloseBracket,
}

impl IndexGet {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.target.span().join(self.close.span())
    }
}

/// An imported declaration.
#[derive(Debug, Clone)]
pub struct UseDecl {
    /// The use token.
    pub use_: Use,
    /// The name of the imported module.
    pub path: Path,
    /// Trailing semi-colon.
    pub semi_colon: SemiColon,
}

/// Parsing an use declaration.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// # fn main() -> anyhow::Result<()> {
/// parse_all::<ast::UseDecl>("use foo;")?;
/// parse_all::<ast::UseDecl>("use foo::bar;")?;
/// parse_all::<ast::UseDecl>("use foo::bar::baz;")?;
/// # Ok(())
/// # }
/// ```
impl Parse for UseDecl {
    fn parse(parser: &mut Parser) -> Result<Self, ParseError> {
        Ok(Self {
            use_: parser.parse()?,
            path: parser.parse()?,
            semi_colon: parser.parse()?,
        })
    }
}

/// A path, where each element is separated by a `::`.
#[derive(Debug, Clone)]
pub struct Path {
    /// The first component in the path.
    pub first: Ident,
    /// The rest of the components in the path.
    pub rest: Vec<(Scope, Ident)>,
}

impl Path {
    /// Convert into an identifier used for instance calls.
    pub fn into_instance_call_ident(self) -> Result<Ident, ParseError> {
        if !self.rest.is_empty() {
            return Err(ParseError::PathCallInstanceError { span: self.span() });
        }

        Ok(self.first)
    }

    /// Calculate the full span of the path.
    pub fn span(&self) -> Span {
        match self.rest.last() {
            Some((_, ident)) => self.first.span().join(ident.span()),
            None => self.first.span(),
        }
    }
}

impl Parse for Path {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let first = parser.parse()?;
        let mut rest = Vec::new();

        while parser.peek::<Scope>()? {
            let scope = parser.parse::<Scope>()?;
            rest.push((scope, parser.parse()?));
        }

        Ok(Self { first, rest })
    }
}

/// A function.
#[derive(Debug, Clone)]
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

impl FnDecl {
    /// Access the span for the function declaration.
    pub fn span(&self) -> Span {
        self.fn_.span().join(self.body.span())
    }
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
#[derive(Debug, Clone)]
pub struct Block {
    /// The close brace.
    pub open: OpenBrace,
    /// Expressions in the block.
    pub exprs: Vec<(Expr, Option<SemiColon>)>,
    /// Test if the expression is trailing.
    pub trailing_expr: Option<Box<Expr>>,
    /// The close brace.
    pub close: CloseBrace,
}

impl Block {
    /// Get the span of the block.
    pub fn span(&self) -> Span {
        self.open.span().join(self.close.span())
    }

    /// Test if the block is empty.
    pub fn produces_nothing(&self) -> bool {
        match &self.trailing_expr {
            Some(trailing) => trailing.produces_nothing(),
            None => true,
        }
    }

    /// Block is literal if a trailing expression exists and is all literal.
    pub fn is_all_literal(&self) -> bool {
        match &self.trailing_expr {
            Some(trailing) => trailing.is_all_literal(),
            None => false,
        }
    }
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
/// assert!(block.trailing_expr.is_none());
///
/// let block = parse_all::<ast::Block>("{ foo }")?.item;
/// assert_eq!(block.exprs.len(), 0);
/// assert!(block.trailing_expr.is_some());
///
/// let block = parse_all::<ast::Block>("{ foo; }")?.item;
/// assert_eq!(block.exprs.len(), 1);
/// assert!(block.trailing_expr.is_none());
///
/// let block = parse_all::<ast::Block>(r#"
///     {
///         let foo = 42;
///         let bar = "string";
///         baz
///     }
/// "#)?.item;
/// assert_eq!(block.exprs.len(), 2);
/// assert!(block.trailing_expr.is_some());
/// # Ok(())
/// # }
/// ```
impl Parse for Block {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let mut exprs = Vec::new();

        let open = parser.parse()?;
        let mut trailing_expr = None;

        // Last expression is of a type that evaluates to a value.
        let mut last_expr_with_value = false;

        while !parser.peek::<CloseBrace>()? {
            last_expr_with_value = false;
            let expr: Expr = parser.parse()?;

            if parser.peek::<SemiColon>()? {
                exprs.push((expr, Some(parser.parse::<SemiColon>()?)));
                continue;
            }

            // expressions where it's allowed not to have a trailing
            // semi-colon.
            match &expr {
                Expr::While(..) => {
                    exprs.push((expr, None));
                    continue;
                }
                Expr::Loop(..) => {
                    exprs.push((expr, None));
                    continue;
                }
                Expr::For(..) => {
                    exprs.push((expr, None));
                    continue;
                }
                Expr::ExprIf(expr_if) => {
                    if expr_if.produces_nothing() {
                        exprs.push((expr, None));
                    } else {
                        last_expr_with_value = true;
                        exprs.push((expr, None));
                    }

                    continue;
                }
                _ => (),
            }

            trailing_expr = Some(Box::new(expr));
            break;
        }

        if last_expr_with_value {
            trailing_expr = exprs.pop().map(|(expr, _)| Box::new(expr));
        }

        let close = parser.parse()?;
        Ok(Block {
            open,
            exprs,
            trailing_expr,
            close,
        })
    }
}

/// Something parenthesized and comma separated `(<T,>*)`.
#[derive(Debug, Clone)]
pub struct FunctionArgs<T> {
    /// The open parenthesis.
    pub open: OpenParen,
    /// The parenthesized type.
    pub items: Vec<T>,
    /// The close parenthesis.
    pub close: CloseParen,
}

impl<T> FunctionArgs<T> {
    /// Access the span of expression.
    pub fn span(&self) -> Span {
        self.open.token.span.join(self.close.token.span)
    }
}

/// Parse function arguments.
///
/// # Examples
///
/// ```rust
/// use rune::{parse_all, ast};
///
/// # fn main() -> anyhow::Result<()> {
/// parse_all::<ast::FunctionArgs<ast::Expr>>("(1, \"two\")")?;
/// parse_all::<ast::FunctionArgs<ast::Expr>>("(1, 2,)")?;
/// parse_all::<ast::FunctionArgs<ast::Expr>>("(1, 2, foo())")?;
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

/// A prioritized expression group `(<expr>)`.
#[derive(Debug, Clone)]
pub struct ExprGroup {
    /// The open parenthesis.
    pub open: OpenParen,
    /// The grouped expression.
    pub expr: Box<Expr>,
    /// The close parenthesis.
    pub close: CloseParen,
}

impl ExprGroup {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        self.open.span().join(self.close.span())
    }

    /// Check if expression is empty.
    pub fn produces_nothing(&self) -> bool {
        self.expr.produces_nothing()
    }
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

/// A return statement `return [expr]`.
#[derive(Debug, Clone)]
pub struct Return {
    /// The return token.
    pub return_: ReturnToken,
    /// An optional expression to return.
    pub expr: Option<Box<Expr>>,
}

impl Return {
    /// Access the span of the expression.
    pub fn span(&self) -> Span {
        if let Some(expr) = &self.expr {
            self.return_.span().join(expr.span())
        } else {
            self.return_.span()
        }
    }
}

impl Parse for Return {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        let return_ = parser.parse()?;

        let expr = if parser.peek::<Expr>()? {
            Some(Box::new(parser.parse()?))
        } else {
            None
        };

        Ok(Self { return_, expr })
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

            impl $parser {
                /// Access the span of the token.
                pub fn span(&self) -> Span {
                    self.token.span
                }
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
                fn peek(p1: Option<Token>, _: Option<Token>) -> bool {
                    match p1 {
                        Some(p1) => match p1.kind {
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
    (Colon, Kind::Colon),
    (Dot, Kind::Dot),
    (SemiColon, Kind::SemiColon),
    (Eq, Kind::Eq),
    (Use, Kind::Use),
    (Scope, Kind::Scope),
    (WhileToken, Kind::While),
    (LoopToken, Kind::Loop),
    (ForToken, Kind::For),
    (InToken, Kind::In),
    (Break, Kind::Break),
    (ReturnToken, Kind::Return),
    (Star, Kind::Star),
    (StartObject, Kind::StartObject),
}

impl<'a> Resolve<'a> for Ident {
    type Output = &'a str;

    fn resolve(&self, source: Source<'a>) -> Result<&'a str, ResolveError> {
        source.source(self.token.span)
    }
}
