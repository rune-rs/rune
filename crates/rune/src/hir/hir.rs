use core::num::NonZeroUsize;

use crate::no_std::borrow::Cow;

use crate as rune;
use crate::ast::{self, Span, Spanned};
use crate::compile;
use crate::parse::{Expectation, Id, IntoExpectation, Opaque, Resolve, ResolveContext};
use crate::runtime::format;

/// Visibility level restricted to some path: pub(self) or pub(super) or pub(crate) or pub(in some::module).
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub(crate) enum Visibility<'hir> {
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
#[derive(Debug, Clone, Copy, PartialEq, Spanned)]
#[non_exhaustive]
pub(crate) struct Pat<'hir> {
    /// The span of the pattern.
    #[rune(span)]
    pub(crate) span: Span,
    /// The kind of the pattern.
    pub(crate) kind: PatKind<'hir>,
}

/// The kind of a [Pat].
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum PatKind<'hir> {
    /// An ignored binding.
    PatIgnore,
    /// The rest pattern `..`.
    PatRest,
    /// A path pattern.
    PatPath(&'hir Path<'hir>),
    /// A literal pattern. This is represented as an expression.
    PatLit(&'hir Expr<'hir>),
    /// A vector pattern.
    PatVec(&'hir PatItems<'hir>),
    /// A tuple pattern.
    PatTuple(&'hir PatItems<'hir>),
    /// An object pattern.
    PatObject(&'hir PatItems<'hir>),
    /// A binding `a: pattern` or `"foo": pattern`.
    PatBinding(&'hir PatBinding<'hir>),
}

/// A tuple pattern.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub(crate) struct PatItems<'hir> {
    /// The path, if the tuple is typed.
    pub(crate) path: Option<&'hir Path<'hir>>,
    /// The items in the tuple.
    pub(crate) items: &'hir [Pat<'hir>],
    /// If the pattern is open.
    pub(crate) is_open: bool,
    /// The number of elements in the pattern.
    pub(crate) count: usize,
}

/// An object item.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub(crate) struct PatBinding<'hir> {
    /// The key of an object.
    pub(crate) key: &'hir ObjectKey<'hir>,
    /// What the binding is to.
    pub(crate) pat: &'hir Pat<'hir>,
}

/// An expression.
#[derive(Debug, Clone, Copy, PartialEq, Spanned)]
#[non_exhaustive]
pub(crate) struct Expr<'hir> {
    /// Span of the expression.
    #[rune(span)]
    pub(crate) span: Span,
    /// The kind of the expression.
    pub(crate) kind: ExprKind<'hir>,
}

/// The kind of a number.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub(crate) enum Lit<'hir> {
    Bool(bool),
    Integer(i64),
    Float(f64),
    Byte(u8),
    Char(char),
    Str(&'hir str),
    ByteStr(&'hir [u8]),
}

/// The kind of an [Expr].
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub(crate) enum ExprKind<'hir> {
    Path(&'hir Path<'hir>),
    Assign(&'hir ExprAssign<'hir>),
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
    Block(&'hir ExprBlock<'hir>),
    Break(Option<&'hir ExprBreakValue<'hir>>),
    Continue(Option<&'hir ast::Label>),
    Yield(Option<&'hir Expr<'hir>>),
    Return(Option<&'hir Expr<'hir>>),
    Await(&'hir Expr<'hir>),
    Try(&'hir Expr<'hir>),
    Select(&'hir ExprSelect<'hir>),
    Closure(&'hir ExprClosure<'hir>),
    Lit(Lit<'hir>),
    Object(&'hir ExprObject<'hir>),
    Tuple(&'hir ExprSeq<'hir>),
    Vec(&'hir ExprSeq<'hir>),
    Range(&'hir ExprRange<'hir>),
    Group(&'hir Expr<'hir>),
    MacroCall(&'hir MacroCall<'hir>),
}

/// A deferred macro call.
///
/// This is used to propagate information on built-in macros to the assembly
/// phase of the compilation.
#[derive(Debug, Clone, Copy, PartialEq, Spanned)]
#[non_exhaustive]
pub(crate) enum MacroCall<'hir> {
    /// The built-in template macro.
    Template(&'hir BuiltInTemplate<'hir>),
    /// The built-in format macro.
    Format(&'hir BuiltInFormat<'hir>),
    /// The built-in file! macro.
    File(&'hir BuiltInFile<'hir>),
    /// The built-in line! macro.
    Line(&'hir BuiltInLine<'hir>),
}

/// An internally resolved template.
#[derive(Debug, Clone, Copy, PartialEq, Spanned)]
pub(crate) struct BuiltInTemplate<'hir> {
    /// The span of the built-in template.
    #[rune(span)]
    pub(crate) span: Span,
    /// Indicate if template originated from literal.
    pub(crate) from_literal: bool,
    /// Expressions being concatenated as a template.
    pub(crate) exprs: &'hir [Expr<'hir>],
}

/// An internal format specification.
#[derive(Debug, Clone, Copy, PartialEq, Spanned)]
pub(crate) struct BuiltInFormat<'hir> {
    #[rune(span)]
    pub(crate) span: Span,
    /// The fill character to use.
    pub(crate) fill: Option<(ast::LitChar, char)>,
    /// Alignment specification.
    pub(crate) align: Option<(ast::Ident, format::Alignment)>,
    /// Width to fill.
    pub(crate) width: Option<(ast::LitNumber, Option<NonZeroUsize>)>,
    /// Precision to fill.
    pub(crate) precision: Option<(ast::LitNumber, Option<NonZeroUsize>)>,
    /// A specification of flags.
    pub(crate) flags: Option<(ast::LitNumber, format::Flags)>,
    /// The format specification type.
    pub(crate) format_type: Option<(ast::Ident, format::Type)>,
    /// The value being formatted.
    pub(crate) value: &'hir Expr<'hir>,
}

/// Macro data for `file!()`
#[derive(Debug, Clone, Copy, PartialEq, Spanned)]
pub(crate) struct BuiltInFile<'hir> {
    /// The span of the built-in-file
    #[rune(span)]
    pub(crate) span: Span,
    /// Path value to use
    pub(crate) value: Lit<'hir>,
}

/// Macro data for `line!()`
#[derive(Debug, Clone, Copy, PartialEq, Spanned)]
pub(crate) struct BuiltInLine<'hir> {
    /// The span of the built-in-file
    #[rune(span)]
    pub(crate) span: Span,
    /// The line number
    pub(crate) value: Lit<'hir>,
}

/// An assign expression `a = b`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub(crate) struct ExprAssign<'hir> {
    /// The expression being assigned to.
    pub(crate) lhs: &'hir Expr<'hir>,
    /// The value.
    pub(crate) rhs: &'hir Expr<'hir>,
}

/// A `loop` expression: `loop { ... }`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub(crate) struct ExprLoop<'hir> {
    /// A label.
    pub(crate) label: Option<&'hir ast::Label>,
    /// A condition to execute the loop, if a condition is necessary.
    pub(crate) condition: Option<&'hir Condition<'hir>>,
    /// The body of the loop.
    pub(crate) body: &'hir Block<'hir>,
}

/// A `for` loop over an iterator: `for i in [1, 2, 3] {}`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub(crate) struct ExprFor<'hir> {
    /// The label of the loop.
    pub(crate) label: Option<&'hir ast::Label>,
    /// The pattern binding to use.
    /// Non-trivial pattern bindings will panic if the value doesn't match.
    pub(crate) binding: &'hir Pat<'hir>,
    /// Expression producing the iterator.
    pub(crate) iter: &'hir Expr<'hir>,
    /// The body of the loop.
    pub(crate) body: &'hir Block<'hir>,
}

/// A let expression `let <name> = <expr>`
#[derive(Debug, Clone, Copy, PartialEq, Spanned)]
#[non_exhaustive]
pub(crate) struct ExprLet<'hir> {
    /// The name of the binding.
    pub(crate) pat: &'hir Pat<'hir>,
    /// The expression the binding is assigned to.
    pub(crate) expr: &'hir Expr<'hir>,
}

/// An if statement: `if cond { true } else { false }`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub(crate) struct ExprIf<'hir> {
    /// The condition to the if statement.
    pub(crate) condition: &'hir Condition<'hir>,
    /// The body of the if statement.
    pub(crate) block: &'hir Block<'hir>,
    /// Else if branches.
    pub(crate) expr_else_ifs: &'hir [ExprElseIf<'hir>],
    /// The else part of the if expression.
    pub(crate) expr_else: Option<&'hir ExprElse<'hir>>,
}

/// An else branch of an if expression.
#[derive(Debug, Clone, Copy, PartialEq, Spanned)]
#[non_exhaustive]
pub(crate) struct ExprElseIf<'hir> {
    /// Span of the expression.
    #[rune(span)]
    pub(crate) span: Span,
    /// The condition for the branch.
    pub(crate) condition: &'hir Condition<'hir>,
    /// The body of the else statement.
    pub(crate) block: &'hir Block<'hir>,
}

/// An else branch of an if expression.
#[derive(Debug, Clone, Copy, PartialEq, Spanned)]
#[non_exhaustive]
pub(crate) struct ExprElse<'hir> {
    /// Span of the expression.
    #[rune(span)]
    pub(crate) span: Span,
    /// The body of the else statement.
    pub(crate) block: &'hir Block<'hir>,
}

/// A match expression.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub(crate) struct ExprMatch<'hir> {
    /// The expression who's result we match over.
    pub(crate) expr: &'hir Expr<'hir>,
    /// Branches.
    pub(crate) branches: &'hir [ExprMatchBranch<'hir>],
}

/// A match branch.
#[derive(Debug, Clone, Copy, PartialEq, Spanned)]
#[non_exhaustive]
pub(crate) struct ExprMatchBranch<'hir> {
    /// Span of the expression.
    #[rune(span)]
    pub(crate) span: Span,
    /// The pattern to match.
    pub(crate) pat: &'hir Pat<'hir>,
    /// The branch condition.
    pub(crate) condition: Option<&'hir Expr<'hir>>,
    /// The body of the match.
    pub(crate) body: &'hir Expr<'hir>,
}

/// A function call `<expr>(<args>)`.
#[derive(Debug, Clone, Copy, PartialEq, Opaque)]
#[non_exhaustive]
pub(crate) struct ExprCall<'hir> {
    /// Opaque identifier related with call.
    #[rune(id)]
    pub(crate) id: Id,
    /// The name of the function being called.
    pub(crate) expr: &'hir Expr<'hir>,
    /// The arguments of the function call.
    pub(crate) args: &'hir [Expr<'hir>],
}

impl<'hir> ExprCall<'hir> {
    /// Get the target of the call expression.
    pub(crate) fn target(&self) -> &Expr {
        if let ExprKind::FieldAccess(access) = self.expr.kind {
            return access.expr;
        }

        self.expr
    }
}

/// A field access `<expr>.<field>`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub(crate) struct ExprFieldAccess<'hir> {
    /// The expr where the field is being accessed.
    pub(crate) expr: &'hir Expr<'hir>,
    /// The field being accessed.
    pub(crate) expr_field: &'hir ExprField<'hir>,
}

/// The field being accessed.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub(crate) enum ExprField<'hir> {
    /// An identifier.
    Path(&'hir Path<'hir>),
    /// A literal number.
    LitNumber(&'hir ast::LitNumber),
}

/// A binary expression.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub(crate) struct ExprBinary<'hir> {
    /// The left-hand side of a binary operation.
    pub(crate) lhs: &'hir Expr<'hir>,
    /// The operator.
    pub(crate) op: ast::BinOp,
    /// The right-hand side of a binary operation.
    pub(crate) rhs: &'hir Expr<'hir>,
}

/// A unary expression.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub(crate) struct ExprUnary<'hir> {
    /// The operation to apply.
    pub(crate) op: ast::UnOp,
    /// The expression of the operation.
    pub(crate) expr: &'hir Expr<'hir>,
}

/// An index get operation `<t>[<index>]`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub(crate) struct ExprIndex<'hir> {
    /// The target of the index set.
    pub(crate) target: &'hir Expr<'hir>,
    /// The indexing expression.
    pub(crate) index: &'hir Expr<'hir>,
}

/// Things that we can break on.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub(crate) enum ExprBreakValue<'hir> {
    /// Breaking a value out of a loop.
    Expr(&'hir Expr<'hir>),
    /// Break and jump to the given label.
    Label(&'hir ast::Label),
}

/// A block expression.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub(crate) struct ExprBlock<'hir> {
    /// The kind of the block.
    pub(crate) kind: ExprBlockKind,
    /// The optional move token.
    pub(crate) block_move: bool,
    /// The close brace.
    pub(crate) block: &'hir Block<'hir>,
}

/// The kind of an [ExprBlock].
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub(crate) enum ExprBlockKind {
    Default,
    Async,
    Const,
}

/// A `select` expression that selects over a collection of futures.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub(crate) struct ExprSelect<'hir> {
    /// The branches of the select.
    pub(crate) branches: &'hir [ExprSelectBranch<'hir>],
}

/// A single selection branch.
#[derive(Debug, Clone, Copy, PartialEq, Spanned)]
#[non_exhaustive]
pub(crate) enum ExprSelectBranch<'hir> {
    /// A patterned branch.
    Pat(&'hir ExprSelectPatBranch<'hir>),
    /// A default branch.
    Default(&'hir Expr<'hir>),
}

/// A single selection branch.
#[derive(Debug, Clone, Copy, PartialEq, Spanned)]
#[non_exhaustive]
pub(crate) struct ExprSelectPatBranch<'hir> {
    /// The identifier to bind the result to.
    pub(crate) pat: &'hir Pat<'hir>,
    /// The expression that should evaluate to a future.
    pub(crate) expr: &'hir Expr<'hir>,
    /// The body of the expression.
    pub(crate) body: &'hir Expr<'hir>,
}

/// A closure expression.
#[derive(Debug, Clone, Copy, PartialEq, Opaque)]
#[non_exhaustive]
pub(crate) struct ExprClosure<'hir> {
    /// Opaque identifier for the closure.
    #[rune(id)]
    pub(crate) id: Id,
    /// Arguments to the closure.
    pub(crate) args: &'hir [FnArg<'hir>],
    /// The body of the closure.
    pub(crate) body: &'hir Expr<'hir>,
}

/// An object expression.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub(crate) struct ExprObject<'hir> {
    /// An object identifier.
    pub(crate) path: Option<&'hir Path<'hir>>,
    /// Assignments in the object.
    pub(crate) assignments: &'hir [FieldAssign<'hir>],
}

/// A single field assignment in an object expression.
#[derive(Debug, Clone, Copy, PartialEq, Spanned)]
#[non_exhaustive]
pub(crate) struct FieldAssign<'hir> {
    /// Span of the field assignment.
    #[rune(span)]
    pub(crate) span: Span,
    /// The key of the field.
    pub(crate) key: &'hir ObjectKey<'hir>,
    /// The assigned expression of the field.
    pub(crate) assign: Option<&'hir Expr<'hir>>,
}

/// Possible literal object keys.
#[derive(Debug, Clone, Copy, PartialEq, Spanned)]
#[non_exhaustive]
pub(crate) enum ObjectKey<'hir> {
    /// A literal string (with escapes).
    LitStr(&'hir ast::LitStr),
    /// A path, usually an identifier.
    Path(&'hir Path<'hir>),
}

impl<'a, 'hir> Resolve<'a> for ObjectKey<'hir> {
    type Output = Cow<'a, str>;

    fn resolve(&self, ctx: ResolveContext<'a>) -> compile::Result<Self::Output> {
        Ok(match *self {
            Self::LitStr(lit_str) => lit_str.resolve(ctx)?,
            Self::Path(path) => {
                let Some(ident) = path.try_as_ident() else {
                    return Err(compile::Error::expected(path, "object key"));
                };

                Cow::Borrowed(ident.resolve(ctx)?)
            }
        })
    }
}

/// A literal vector.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub(crate) struct ExprSeq<'hir> {
    /// Items in the vector.
    pub(crate) items: &'hir [Expr<'hir>],
}

/// A range expression `a .. b` or `a ..= b`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub(crate) struct ExprRange<'hir> {
    /// Start of range.
    pub(crate) from: Option<&'hir Expr<'hir>>,
    /// The range limits.
    pub(crate) limits: ExprRangeLimits,
    /// End of range.
    pub(crate) to: Option<&'hir Expr<'hir>>,
}

/// The limits of the specified range.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub(crate) enum ExprRangeLimits {
    /// Half-open range expression.
    HalfOpen,
    /// Closed expression.
    Closed,
}

/// The condition in an if statement.
#[derive(Debug, Clone, Copy, PartialEq, Spanned)]
#[non_exhaustive]
pub(crate) enum Condition<'hir> {
    /// A regular expression.
    Expr(&'hir Expr<'hir>),
    /// A pattern match.
    ExprLet(&'hir ExprLet<'hir>),
}

/// A path.
#[derive(Debug, Clone, Copy, PartialEq, Opaque, Spanned)]
#[non_exhaustive]
pub(crate) struct Path<'hir> {
    /// Opaque id associated with path.
    #[rune(id)]
    pub(crate) id: Id,
    /// The span of the path.
    #[rune(span)]
    pub(crate) span: Span,
    /// The span of the global indicator.
    pub(crate) global: Option<Span>,
    /// The span of the trailing indicator.
    pub(crate) trailing: Option<Span>,
    /// The first component in the path.
    pub(crate) first: &'hir PathSegment<'hir>,
    /// The rest of the components in the path.
    pub(crate) rest: &'hir [PathSegment<'hir>],
}

impl<'hir> Path<'hir> {
    /// Identify the kind of the path.
    pub(crate) fn as_kind(&self) -> Option<ast::PathKind<'_>> {
        if self.rest.is_empty() && self.trailing.is_none() && self.global.is_none() {
            match self.first.kind {
                PathSegmentKind::SelfValue => Some(ast::PathKind::SelfValue),
                PathSegmentKind::Ident(ident) => Some(ast::PathKind::Ident(ident)),
                _ => None,
            }
        } else {
            None
        }
    }

    /// Borrow as an identifier used for field access calls.
    ///
    /// This is only allowed if there are no other path components
    /// and the path segment is not `Crate` or `Super`.
    pub(crate) fn try_as_ident(&self) -> Option<&'hir ast::Ident> {
        if self.rest.is_empty() && self.trailing.is_none() && self.global.is_none() {
            self.first.try_as_ident()
        } else {
            None
        }
    }

    /// Borrow ident and generics at the same time.
    pub(crate) fn try_as_ident_generics(
        &self,
    ) -> Option<(&ast::Ident, Option<(Span, &'hir [Expr<'hir>])>)> {
        if self.trailing.is_none() && self.global.is_none() {
            if let Some(ident) = self.first.try_as_ident() {
                let generics = if let [PathSegment {
                    span,
                    kind: PathSegmentKind::Generics(generics),
                    ..
                }] = *self.rest
                {
                    Some((span, generics))
                } else {
                    None
                };

                return Some((ident, generics));
            }
        }

        None
    }
}

impl IntoExpectation for &Path<'_> {
    fn into_expectation(self) -> Expectation {
        Expectation::Description("path")
    }
}

/// A single path segment.
#[derive(Debug, Clone, Copy, PartialEq, Spanned)]
#[non_exhaustive]
pub(crate) struct PathSegment<'hir> {
    /// The span of the path segment.
    #[rune(span)]
    pub(crate) span: Span,
    /// The kind of the path segment.
    pub(crate) kind: PathSegmentKind<'hir>,
}

impl<'hir> PathSegment<'hir> {
    /// Borrow as an identifier.
    ///
    /// This is only allowed if the PathSegment is `Ident(_)`
    /// and not `Crate` or `Super`.
    pub(crate) fn try_as_ident(&self) -> Option<&ast::Ident> {
        if let PathSegmentKind::Ident(ident) = self.kind {
            Some(ident)
        } else {
            None
        }
    }
}

/// A single segment in a path.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub(crate) enum PathSegmentKind<'hir> {
    /// A path segment that contains `Self`.
    SelfType,
    /// A path segment that contains `self`.
    SelfValue,
    /// A path segment that is an identifier.
    Ident(&'hir ast::Ident),
    /// The `crate` keyword used as a path segment.
    Crate,
    /// The `super` keyword use as a path segment.
    Super,
    /// A path segment that is a generic argument.
    Generics(&'hir [Expr<'hir>]),
}

#[derive(Debug, Clone, Copy, PartialEq, Opaque, Spanned)]
#[non_exhaustive]
pub(crate) struct ItemFn<'hir> {
    /// Opaque identifier for fn item.
    #[rune(id)]
    pub(crate) id: Id,
    /// The span of the function.
    #[rune(span)]
    pub(crate) span: Span,
    /// The visibility of the `fn` item
    pub(crate) visibility: &'hir Visibility<'hir>,
    /// The name of the function.
    pub(crate) name: &'hir ast::Ident,
    /// The arguments of the function.
    pub(crate) args: &'hir [FnArg<'hir>],
    /// The body of the function.
    pub(crate) body: &'hir Block<'hir>,
}

/// A single argument to a function.
#[derive(Debug, Clone, Copy, PartialEq, Spanned)]
#[non_exhaustive]
pub(crate) enum FnArg<'hir> {
    /// The `self` parameter.
    SelfValue(Span),
    /// Function argument is a pattern binding.
    Pat(&'hir Pat<'hir>),
}

/// A block of statements.
#[derive(Debug, Clone, Copy, PartialEq, Opaque, Spanned)]
#[non_exhaustive]
pub(crate) struct Block<'hir> {
    /// The unique identifier for the block expression.
    #[rune(id)]
    pub(crate) id: Id,
    /// The span of the block.
    #[rune(span)]
    pub(crate) span: Span,
    /// Statements in the block.
    pub(crate) statements: &'hir [Stmt<'hir>],
}

impl Block<'_> {
    /// Test if the block doesn't produce anything. Which is when the last
    /// element is either a non-expression or is an expression terminated by a
    /// semi.
    pub(crate) fn produces_nothing(&self) -> bool {
        matches!(self.statements.last(), Some(Stmt::Semi(..)) | None)
    }
}

/// A statement within a block.
#[derive(Debug, Clone, Copy, PartialEq, Spanned)]
#[non_exhaustive]
pub(crate) enum Stmt<'hir> {
    /// A local declaration.
    Local(&'hir Local<'hir>),
    /// An expression.
    Expr(&'hir Expr<'hir>),
    /// An expression with a trailing semi-colon.
    Semi(&'hir Expr<'hir>),
    /// An ignored item.
    Item(Span),
}

/// A local variable declaration `let <pattern> = <expr>;`
#[derive(Debug, Clone, Copy, PartialEq, Spanned)]
#[non_exhaustive]
pub(crate) struct Local<'hir> {
    /// The span of the local declaration.
    #[rune(span)]
    pub(crate) span: Span,
    /// The name of the binding.
    pub(crate) pat: &'hir Pat<'hir>,
    /// The expression the binding is assigned to.
    pub(crate) expr: &'hir Expr<'hir>,
}
