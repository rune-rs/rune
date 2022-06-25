use crate::ast::{self, Span};
use crate::parse::{Id, Opaque};

/// An identifier, like `foo` or `Hello`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ident {
    /// The source of the identifier.
    pub source: ast::LitSource,
}

/// A label, like `'foo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Label {
    /// The span of the label.
    pub span: Span,
    /// The source of the label.
    pub source: ast::LitSource,
}

/// Visibility level restricted to some path: pub(self) or pub(super) or pub or pub(in some::module).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Visibility<'hir> {
    /// An inherited visibility level, this usually means private.
    Inherited,
    /// An unrestricted public visibility level: `pub`.
    Public,
    /// Crate visibility `pub`.
    Crate,
    /// Super visibility `pub(super)`.
    Super,
    /// Self visibility `pub(self)`.
    SelfValue,
    /// In visibility `pub(in path)`.
    In(&'hir Path<'hir>),
}

/// A pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Pat<'hir> {
    /// An ignored binding.
    PatIgnore(&'hir PatIgnore),
    /// A path pattern.
    PatPath(&'hir PatPath<'hir>),
    /// A literal pattern. This is represented as an expression.
    PatLit(&'hir PatLit<'hir>),
    /// A vector pattern.
    PatVec(&'hir PatVec<'hir>),
    /// A tuple pattern.
    PatTuple(&'hir PatTuple<'hir>),
    /// An object pattern.
    PatObject(&'hir PatObject<'hir>),
    /// A binding `a: pattern` or `"foo": pattern`.
    PatBinding(&'hir PatBinding<'hir>),
    /// The rest pattern `..`.
    PatRest,
}

/// An ignore pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct PatIgnore {
    pub span: Span,
}

/// A path pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct PatPath<'hir> {
    /// The path of the pattern.
    pub path: &'hir Path<'hir>,
}

/// A literal pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct PatLit<'hir> {
    /// The literal expression.
    pub expr: &'hir Expr<'hir>,
}

/// An array pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct PatVec<'hir> {
    /// Bracketed patterns.
    pub items: &'hir [Pat<'hir>],
}

/// A tuple pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct PatTuple<'hir> {
    /// The path, if the tuple is typed.
    pub path: Option<&'hir Path<'hir>>,
    /// The items in the tuple.
    pub items: &'hir [Pat<'hir>],
}

/// An object pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct PatObject<'hir> {
    /// The identifier of the object pattern.
    pub ident: &'hir ObjectIdent<'hir>,
    /// The fields matched against.
    pub items: &'hir [Pat<'hir>],
}

/// An object item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct PatBinding<'hir> {
    /// The key of an object.
    pub key: &'hir ObjectKey<'hir>,
    /// What the binding is to.
    pub pat: &'hir Pat<'hir>,
}

/// An hir expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expr<'hir> {
    Path(&'hir Path<'hir>),
    Assign(&'hir ExprAssign<'hir>),
    While(&'hir ExprWhile<'hir>),
    Loop(&'hir ExprLoop<'hir>),
    For(&'hir ExprFor<'hir>),
    Let(&'hir ExprLet<'hir>),
    If(&'hir ExprIf<'hir>),
    Match(&'hir ExprMatch<'hir>),
    Call(&'hir ExprCall<'hir>),
    FieldAccess(&'hir ExprFieldAccess<'hir>),
    Binary(&'hir ExprBinary<'hir>),
    Unary(&'hir ExprUnary<'hir>),
    Index(&'hir ExprIndex<'hir>),
    Break(&'hir ExprBreak<'hir>),
    Continue(&'hir ExprContinue<'hir>),
    Yield(&'hir ExprYield<'hir>),
    Block(&'hir ExprBlock<'hir>),
    Return(&'hir ExprReturn<'hir>),
    Await(&'hir ExprAwait<'hir>),
    Try(&'hir ExprTry<'hir>),
    Select(&'hir ExprSelect<'hir>),
    Closure(&'hir ExprClosure<'hir>),
    Lit(&'hir ExprLit<'hir>),
    Object(&'hir ExprObject<'hir>),
    Tuple(&'hir ExprTuple<'hir>),
    Vec(&'hir ExprVec<'hir>),
    Range(&'hir ExprRange<'hir>),
    Group(&'hir Expr<'hir>),
    /// Ignored expression.
    Ignore,
}

/// An assign expression `a = b`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprAssign<'hir> {
    /// The expression being assigned to.
    pub lhs: &'hir Expr<'hir>,
    /// The value.
    pub rhs: &'hir Expr<'hir>,
}

/// A `while` loop: `while [expr] { ... }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprWhile<'hir> {
    /// A label for the while loop.
    pub label: Option<&'hir Label>,
    /// The name of the binding.
    pub condition: &'hir Condition<'hir>,
    /// The body of the while loop.
    pub body: &'hir Block<'hir>,
}

/// A `loop` expression: `loop { ... }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprLoop<'hir> {
    /// A label.
    pub label: Option<&'hir Label>,
    /// The body of the loop.
    pub body: &'hir Block<'hir>,
}

/// A `for` loop over an iterator: `for i in [1, 2, 3] {}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprFor<'hir> {
    /// The label of the loop.
    pub label: Option<&'hir Label>,
    /// The pattern binding to use.
    /// Non-trivial pattern bindings will panic if the value doesn't match.
    pub binding: &'hir Pat<'hir>,
    /// Expression producing the iterator.
    pub iter: &'hir Expr<'hir>,
    /// The body of the loop.
    pub body: &'hir Block<'hir>,
}

/// A let expression `let <name> = <expr>`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprLet<'hir> {
    /// The name of the binding.
    pub pat: &'hir Pat<'hir>,
    /// The expression the binding is assigned to.
    pub expr: &'hir Expr<'hir>,
}

/// An if statement: `if cond { true } else { false }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprIf<'hir> {
    /// The condition to the if statement.
    pub condition: &'hir Condition<'hir>,
    /// The body of the if statement.
    pub block: &'hir Block<'hir>,
    /// Else if branches.
    pub expr_else_ifs: &'hir [ExprElseIf<'hir>],
    /// The else part of the if expression.
    pub expr_else: Option<&'hir ExprElse<'hir>>,
}

/// An else branch of an if expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprElseIf<'hir> {
    /// The condition for the branch.
    pub condition: &'hir Condition<'hir>,
    /// The body of the else statement.
    pub block: &'hir Block<'hir>,
}

/// An else branch of an if expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprElse<'hir> {
    /// The body of the else statement.
    pub block: &'hir Block<'hir>,
}

/// A match expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprMatch<'hir> {
    /// The expression who's result we match over.
    pub expr: &'hir Expr<'hir>,
    /// Branches.
    pub branches: &'hir [ExprMatchBranch<'hir>],
}

/// A match branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprMatchBranch<'hir> {
    /// The pattern to match.
    pub pat: &'hir Pat<'hir>,
    /// The branch condition.
    pub condition: Option<&'hir Expr<'hir>>,
    /// The body of the match.
    pub body: &'hir Expr<'hir>,
}

/// A function call `<expr>(<args>)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Opaque)]
#[non_exhaustive]
pub struct ExprCall<'hir> {
    /// Opaque identifier related with call.
    #[rune(id)]
    pub(crate) id: Id,
    /// The name of the function being called.
    pub expr: &'hir Expr<'hir>,
    /// The arguments of the function call.
    pub args: &'hir [Expr<'hir>],
}

/// A field access `<expr>.<field>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprFieldAccess<'hir> {
    /// The expr where the field is being accessed.
    pub expr: &'hir Expr<'hir>,
    /// The field being accessed.
    pub expr_field: &'hir ExprField<'hir>,
}

/// The field being accessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExprField<'hir> {
    /// An identifier.
    Path(&'hir Path<'hir>),
    /// A literal number.
    LitNumber(&'hir ast::LitNumber),
}

/// A binary expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprBinary<'hir> {
    /// The left-hand side of a binary operation.
    pub lhs: &'hir Expr<'hir>,
    /// The operator.
    pub op: ast::BinOp,
    /// The right-hand side of a binary operation.
    pub rhs: &'hir Expr<'hir>,
}

/// A unary expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprUnary<'hir> {
    /// The operation to apply.
    pub op: ast::UnOp,
    /// The expression of the operation.
    pub expr: &'hir Expr<'hir>,
}

/// An index get operation `<target>[<index>]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprIndex<'hir> {
    /// The target of the index set.
    pub target: &'hir Expr<'hir>,
    /// The indexing expression.
    pub index: &'hir Expr<'hir>,
}

/// A `break` statement: `break [expr]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprBreak<'hir> {
    /// An optional expression to break with.
    pub expr: Option<&'hir ExprBreakValue<'hir>>,
}

/// Things that we can break on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExprBreakValue<'hir> {
    /// Breaking a value out of a loop.
    Expr(&'hir Expr<'hir>),
    /// Break and jump to the given label.
    Label(&'hir Label),
}

/// A `continue` statement: `continue [label]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprContinue<'hir> {
    /// An optional label to continue to.
    pub label: Option<&'hir Label>,
}

/// A `yield [expr]` expression to return a value from a generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprYield<'hir> {
    /// An optional expression to yield.
    pub expr: Option<&'hir Expr<'hir>>,
}

/// A block expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprBlock<'hir> {
    /// The kind of the block.
    pub kind: ExprBlockKind,
    /// The optional move token.
    pub block_move: bool,
    /// The close brace.
    pub block: &'hir Block<'hir>,
}

/// The kind of an [ExprBlock].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExprBlockKind {
    Default,
    Async,
    Const,
}

/// A return expression `return [expr]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprReturn<'hir> {
    /// An optional expression to return.
    pub expr: Option<&'hir Expr<'hir>>,
}

/// A return statement `<expr>.await`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprAwait<'hir> {
    /// The expression being awaited.
    pub expr: &'hir Expr<'hir>,
}

/// A try expression `<expr>?`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprTry<'hir> {
    /// The expression being awaited.
    pub expr: &'hir Expr<'hir>,
}

/// A `select` expression that selects over a collection of futures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprSelect<'hir> {
    /// The branches of the select.
    pub branches: &'hir [ExprSelectBranch<'hir>],
}

/// A single selection branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExprSelectBranch<'hir> {
    /// A patterned branch.
    Pat(&'hir ExprSelectPatBranch<'hir>),
    /// A default branch.
    Default(&'hir ExprDefaultBranch<'hir>),
}

/// A single selection branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprDefaultBranch<'hir> {
    /// The body of the expression.
    pub body: &'hir Expr<'hir>,
}

/// A single selection branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprSelectPatBranch<'hir> {
    /// The identifier to bind the result to.
    pub pat: &'hir Pat<'hir>,
    /// The expression that should evaluate to a future.
    pub expr: &'hir Expr<'hir>,
    /// The body of the expression.
    pub body: &'hir Expr<'hir>,
}

/// A closure expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Opaque)]
#[non_exhaustive]
pub struct ExprClosure<'hir> {
    /// Opaque identifier for the closure.
    #[rune(id)]
    pub(crate) id: Id,
    /// Arguments to the closure.
    pub args: &'hir [FnArg<'hir>],
    /// The body of the closure.
    pub body: &'hir Expr<'hir>,
}

/// A literal expression. With the addition of being able to receive attributes,
/// this is identical to [ast::Lit].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprLit<'hir> {
    /// The literal in the expression.
    pub lit: &'hir ast::Lit,
}

/// An object expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprObject<'hir> {
    /// An object identifier.
    pub ident: &'hir ObjectIdent<'hir>,
    /// Assignments in the object.
    pub assignments: &'hir [FieldAssign<'hir>],
}

/// A single field assignment in an object expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct FieldAssign<'hir> {
    /// The key of the field.
    pub key: &'hir ObjectKey<'hir>,
    /// The assigned expression of the field.
    pub assign: Option<&'hir Expr<'hir>>,
}

/// Possible literal object keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ObjectKey<'hir> {
    /// A literal string (with escapes).
    LitStr(&'hir ast::LitStr),
    /// A path, usually an identifier.
    Path(&'hir Path<'hir>),
}

/// A literal object identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ObjectIdent<'hir> {
    /// An anonymous object.
    Anonymous,
    /// A named object.
    Named(Path<'hir>),
}

/// An expression to construct a literal tuple.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprTuple<'hir> {
    /// Items in the tuple.
    pub items: &'hir [Expr<'hir>],
}

/// A literal vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprVec<'hir> {
    /// Items in the vector.
    pub items: &'hir [Expr<'hir>],
}

/// A range expression `a .. b` or `a ..= b`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExprRange<'hir> {
    /// Start of range.
    pub from: Option<&'hir Expr<'hir>>,
    /// The range limits.
    pub limits: ExprRangeLimits,
    /// End of range.
    pub to: Option<&'hir Expr<'hir>>,
}

/// The limits of the specified range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExprRangeLimits {
    /// Half-open range expression.
    HalfOpen,
    /// Closed expression.
    Closed,
}

/// The condition in an if statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Condition<'hir> {
    /// A regular expression.
    Expr(&'hir Expr<'hir>),
    /// A pattern match.
    ExprLet(&'hir ExprLet<'hir>),
}

/// A path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Path<'hir> {
    /// Opaque id associated with path.
    pub id: Id,
    /// The first component in the path.
    pub first: PathSegment<'hir>,
    /// The rest of the components in the path.
    pub rest: &'hir [PathSegment<'hir>],
}

/// A single segment in a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PathSegment<'hir> {
    /// A path segment that contains `Self`.
    SelfType,
    /// A path segment that contains `self`.
    SelfValue,
    /// A path segment that is an identifier.
    Ident(&'hir Ident),
    /// The `crate` keyword used as a path segment.
    Crate,
    /// The `super` keyword use as a path segment.
    Super,
    /// A path segment that is a generic argument.
    Generics(&'hir [Expr<'hir>]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Opaque)]
#[non_exhaustive]
pub struct ItemFn<'hir> {
    /// Opaque identifier for fn item.
    #[rune(id)]
    pub id: Id,
    /// The visibility of the `fn` item
    pub visibility: &'hir Visibility<'hir>,
    /// The name of the function.
    pub name: &'hir Ident,
    /// The arguments of the function.
    pub args: &'hir [FnArg<'hir>],
    /// The body of the function.
    pub body: &'hir Block<'hir>,
}

/// A single argument to a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FnArg<'hir> {
    /// The `self` parameter.
    SelfValue,
    /// Function argument is a pattern binding.
    Pat(&'hir Pat<'hir>),
}

/// A block of statements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Opaque)]
#[non_exhaustive]
pub struct Block<'hir> {
    /// The unique identifier for the block expression.
    #[rune(id)]
    pub id: Id,
    /// The span of the block.
    pub span: Span,
    /// Statements in the block.
    pub statements: &'hir [Stmt<'hir>],
}

/// A statement within a block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Stmt<'hir> {
    /// A local declaration.
    Local(&'hir Local<'hir>),
    /// An expression.
    Expr(&'hir Expr<'hir>),
    /// An expression with a trailing semi-colon.
    Semi(&'hir Expr<'hir>),
    /// An ignored statement.
    Ignore,
}

/// A local variable declaration `let <pattern> = <expr>;`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Local<'hir> {
    /// The name of the binding.
    pub pat: &'hir Pat<'hir>,
    /// The expression the binding is assigned to.
    pub expr: &'hir Expr<'hir>,
}
