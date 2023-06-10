use core::num::NonZeroUsize;

use crate as rune;
use crate::ast::{self, Span, Spanned};
use crate::compile::{ItemId, ModId};
use crate::hir::Name;
use crate::parse::{Expectation, Id, IntoExpectation, NonZeroId};
use crate::runtime::{format, Type, TypeCheck};
use crate::Hash;

/// A pattern.
#[derive(Debug, Clone, Copy, Spanned)]
#[non_exhaustive]
pub(crate) struct Pat<'hir> {
    /// The span of the pattern.
    #[rune(span)]
    pub(crate) span: Span,
    /// The kind of the pattern.
    pub(crate) kind: PatKind<'hir>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum PatPathKind<'hir> {
    Kind(&'hir PatItemsKind),
    Ident(&'hir str),
}

/// The kind of a [Pat].
#[derive(Debug, Clone, Copy)]
pub(crate) enum PatKind<'hir> {
    /// An ignored binding.
    Ignore,
    /// The rest pattern `..`.
    Rest,
    /// A path pattern.
    Path(&'hir PatPathKind<'hir>),
    /// A literal pattern. This is represented as an expression.
    Lit(&'hir Expr<'hir>),
    /// A vector pattern.
    Vec(&'hir PatItems<'hir>),
    /// A tuple pattern.
    Tuple(&'hir PatItems<'hir>),
    /// An object pattern.
    Object(&'hir PatItems<'hir>),
    /// A binding `a: pattern` or `"foo": pattern`.
    Binding,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum PatItemsKind {
    Type {
        hash: Hash,
    },
    BuiltInVariant {
        type_check: TypeCheck,
    },
    Variant {
        variant_hash: Hash,
        enum_hash: Hash,
        index: usize,
    },
    Anonymous {
        count: usize,
        is_open: bool,
    },
}

/// Items pattern matching.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub(crate) struct PatItems<'hir> {
    /// The kind of pattern items.
    pub(crate) kind: PatItemsKind,
    /// The items in the tuple.
    pub(crate) items: &'hir [Pat<'hir>],
    /// If the pattern is open.
    pub(crate) is_open: bool,
    /// The number of elements in the pattern.
    pub(crate) count: usize,
    /// Bindings associated with the pattern.
    pub(crate) bindings: &'hir [Binding<'hir>],
}

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub(crate) enum Binding<'hir> {
    Binding(Span, &'hir str, &'hir Pat<'hir>),
    Ident(Span, &'hir str),
}

impl<'hir> Spanned for Binding<'hir> {
    fn span(&self) -> Span {
        match self {
            Binding::Binding(span, _, _) => *span,
            Binding::Ident(span, _) => *span,
        }
    }
}

impl<'hir> Binding<'hir> {
    pub(crate) fn key(&self) -> &'hir str {
        match *self {
            Self::Binding(_, key, _) => key,
            Self::Ident(_, key) => key,
        }
    }
}

/// An expression.
#[derive(Debug, Clone, Copy, Spanned)]
#[non_exhaustive]
pub(crate) struct Expr<'hir> {
    /// Span of the expression.
    #[rune(span)]
    pub(crate) span: Span,
    /// The kind of the expression.
    pub(crate) kind: ExprKind<'hir>,
}

/// The kind of a number.
#[derive(Debug, Clone, Copy)]
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
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub(crate) enum ExprKind<'hir> {
    Variable(Name<'hir>),
    Type(Type),
    Fn(Hash),
    Path(&'hir Path<'hir>),
    Assign(&'hir ExprAssign<'hir>),
    Loop(&'hir ExprLoop<'hir>),
    For(&'hir ExprFor<'hir>),
    Let(&'hir ExprLet<'hir>),
    If(&'hir Conditional<'hir>),
    Match(&'hir ExprMatch<'hir>),
    Call(&'hir ExprCall<'hir>),
    FieldAccess(&'hir ExprFieldAccess<'hir>),
    Binary(&'hir ExprBinary<'hir>),
    Unary(&'hir ExprUnary<'hir>),
    Index(&'hir ExprIndex<'hir>),
    AsyncBlock(&'hir ExprAsyncBlock<'hir>),
    Block(&'hir Block<'hir>),
    Break(Option<&'hir ExprBreakValue<'hir>>),
    Continue(Option<&'hir ast::Label>),
    Yield(Option<&'hir Expr<'hir>>),
    Return(Option<&'hir Expr<'hir>>),
    Await(&'hir Expr<'hir>),
    Try(&'hir Expr<'hir>),
    Select(&'hir ExprSelect<'hir>),
    CallClosure(&'hir ExprCallClosure<'hir>),
    Lit(Lit<'hir>),
    Object(&'hir ExprObject<'hir>),
    Tuple(&'hir ExprSeq<'hir>),
    Vec(&'hir ExprSeq<'hir>),
    Range(&'hir ExprRange<'hir>),
    Group(&'hir Expr<'hir>),
    Template(&'hir BuiltInTemplate<'hir>),
    Format(&'hir BuiltInFormat<'hir>),
    Const(Hash),
}

/// An internally resolved template.
#[derive(Debug, Clone, Copy, Spanned)]
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
#[derive(Debug, Clone, Copy, Spanned)]
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

/// An assign expression `a = b`.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub(crate) struct ExprAssign<'hir> {
    /// The expression being assigned to.
    pub(crate) lhs: &'hir Expr<'hir>,
    /// The value.
    pub(crate) rhs: &'hir Expr<'hir>,
}

/// A `loop` expression: `loop { ... }`.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub(crate) struct ExprLoop<'hir> {
    /// A label.
    pub(crate) label: Option<&'hir ast::Label>,
    /// A condition to execute the loop, if a condition is necessary.
    pub(crate) condition: Option<&'hir Condition<'hir>>,
    /// The body of the loop.
    pub(crate) body: &'hir Block<'hir>,
    /// Variables that have been defined by the loop header.
    #[allow(unused)]
    pub(crate) drop: &'hir [Name<'hir>],
}

/// A `for` loop over an iterator: `for i in [1, 2, 3] {}`.
#[derive(Debug, Clone, Copy)]
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
    /// Variables that have been defined by the loop header.
    #[allow(unused)]
    pub(crate) drop: &'hir [Name<'hir>],
}

/// A let expression `let <name> = <expr>`
#[derive(Debug, Clone, Copy, Spanned)]
#[non_exhaustive]
pub(crate) struct ExprLet<'hir> {
    /// The name of the binding.
    pub(crate) pat: &'hir Pat<'hir>,
    /// The expression the binding is assigned to.
    pub(crate) expr: &'hir Expr<'hir>,
}

/// A sequence of conditional branches.
///
/// This is lower from if statements, such as:
///
/// ```text
/// if cond { true } else { false }
/// ```
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub(crate) struct Conditional<'hir> {
    /// Else if branches.
    pub(crate) branches: &'hir [ConditionalBranch<'hir>],
}

/// An else branch of an if expression.
#[derive(Debug, Clone, Copy, Spanned)]
#[non_exhaustive]
pub(crate) struct ConditionalBranch<'hir> {
    /// Span of the expression.
    #[rune(span)]
    pub(crate) span: Span,
    /// The condition for the branch. Empty condition means that this is the
    /// fallback branch.
    pub(crate) condition: Option<&'hir Condition<'hir>>,
    /// The body of the else statement.
    pub(crate) block: &'hir Block<'hir>,
    /// Variables that have been defined by the conditional header.
    #[allow(unused)]
    pub(crate) drop: &'hir [Name<'hir>],
}

/// A match expression.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub(crate) struct ExprMatch<'hir> {
    /// The expression who's result we match over.
    pub(crate) expr: &'hir Expr<'hir>,
    /// Branches.
    pub(crate) branches: &'hir [ExprMatchBranch<'hir>],
}

/// A match branch.
#[derive(Debug, Clone, Copy, Spanned)]
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
    /// Variables that have been defined by this match branch, which needs to be
    /// dropped.
    #[allow(unused)]
    pub(crate) drop: &'hir [Name<'hir>],
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Call<'hir> {
    Var {
        /// The name of the variable being called.
        name: Name<'hir>,
    },
    Instance {
        /// The target expression being called.
        target: &'hir Expr<'hir>,
        /// Hash of the fn being called.
        hash: Hash,
    },
    Meta {
        /// Hash being called.
        hash: Hash,
    },
    /// An expression being called.
    Expr { expr: &'hir Expr<'hir> },
    /// A constant function call.
    ConstFn {
        /// The identifier of the constant function.
        id: NonZeroId,
        /// Ast identifier.
        ast_id: Id,
    },
}

/// A function call `<expr>(<args>)`.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub(crate) struct ExprCall<'hir> {
    /// The call being performed.
    pub(crate) call: Call<'hir>,
    /// The arguments of the function call.
    pub(crate) args: &'hir [Expr<'hir>],
}

/// A field access `<expr>.<field>`.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub(crate) struct ExprFieldAccess<'hir> {
    /// The expr where the field is being accessed.
    pub(crate) expr: &'hir Expr<'hir>,
    /// The field being accessed.
    pub(crate) expr_field: &'hir ExprField<'hir>,
}

/// The field being accessed.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub(crate) enum ExprField<'hir> {
    /// An identifier.
    Path(&'hir Path<'hir>),
    /// A literal number.
    LitNumber(&'hir ast::LitNumber),
}

/// A binary expression.
#[derive(Debug, Clone, Copy)]
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
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub(crate) struct ExprUnary<'hir> {
    /// The operation to apply.
    pub(crate) op: ast::UnOp,
    /// The expression of the operation.
    pub(crate) expr: &'hir Expr<'hir>,
}

/// An index get operation `<t>[<index>]`.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub(crate) struct ExprIndex<'hir> {
    /// The target of the index set.
    pub(crate) target: &'hir Expr<'hir>,
    /// The indexing expression.
    pub(crate) index: &'hir Expr<'hir>,
}

/// Things that we can break on.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub(crate) enum ExprBreakValue<'hir> {
    /// Breaking a value out of a loop.
    Expr(&'hir Expr<'hir>),
    /// Break and jump to the given label.
    Label(&'hir ast::Label),
}

/// An async block being called.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub(crate) struct ExprAsyncBlock<'hir> {
    pub(crate) hash: Hash,
    pub(crate) do_move: bool,
    pub(crate) captures: &'hir [Name<'hir>],
}

/// A `select` expression that selects over a collection of futures.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub(crate) struct ExprSelect<'hir> {
    /// The branches of the select.
    pub(crate) branches: &'hir [ExprSelectBranch<'hir>],
}

/// A single selection branch.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub(crate) enum ExprSelectBranch<'hir> {
    /// A patterned branch.
    Pat(&'hir ExprSelectPatBranch<'hir>),
    /// A default branch.
    Default(&'hir Expr<'hir>),
}

/// A single selection branch.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub(crate) struct ExprSelectPatBranch<'hir> {
    /// The identifier to bind the result to.
    pub(crate) pat: &'hir Pat<'hir>,
    /// The expression that should evaluate to a future.
    pub(crate) expr: &'hir Expr<'hir>,
    /// The body of the expression.
    pub(crate) body: &'hir Expr<'hir>,
    /// Variables that need to be dropped by the end of this block.
    #[allow(unused)]
    pub(crate) drop: &'hir [Name<'hir>],
}

/// Calling a closure.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub(crate) struct ExprCallClosure<'hir> {
    pub(crate) do_move: bool,
    pub(crate) hash: Hash,
    pub(crate) captures: &'hir [Name<'hir>],
}

/// A closure expression.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub(crate) struct ExprClosure<'hir> {
    /// Arguments to the closure.
    pub(crate) args: &'hir [FnArg<'hir>],
    /// The body of the closure.
    pub(crate) body: &'hir Expr<'hir>,
    /// Captures in the closure.
    pub(crate) captures: &'hir [Name<'hir>],
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ExprObjectKind {
    UnitStruct { hash: Hash },
    Struct { hash: Hash },
    StructVariant { hash: Hash },
    Anonymous,
}

/// An object expression.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub(crate) struct ExprObject<'hir> {
    /// The kind of an object being created.
    pub(crate) kind: ExprObjectKind,
    /// Assignments in the object.
    pub(crate) assignments: &'hir [FieldAssign<'hir>],
}

/// A single field assignment in an object expression.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub(crate) struct FieldAssign<'hir> {
    /// The key of the field.
    pub(crate) key: (Span, &'hir str),
    /// The assigned expression of the field.
    pub(crate) assign: &'hir Expr<'hir>,
}

/// A literal vector.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub(crate) struct ExprSeq<'hir> {
    /// Items in the vector.
    pub(crate) items: &'hir [Expr<'hir>],
}

/// A range expression `a .. b` or `a ..= b`.
#[derive(Debug, Clone, Copy)]
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
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub(crate) enum ExprRangeLimits {
    /// Half-open range expression.
    HalfOpen,
    /// Closed expression.
    Closed,
}

/// The condition in an if statement.
#[derive(Debug, Clone, Copy, Spanned)]
#[non_exhaustive]
pub(crate) enum Condition<'hir> {
    /// A regular expression.
    Expr(&'hir Expr<'hir>),
    /// A pattern match.
    ExprLet(&'hir ExprLet<'hir>),
}

/// A path.
#[derive(Debug, Clone, Copy, Spanned)]
#[non_exhaustive]
pub(crate) struct Path<'hir> {
    /// The span of the path.
    #[rune(span)]
    pub(crate) span: Span,
    /// The module the path belongs to.
    pub(crate) module: ModId,
    /// The item the path belongs to.
    pub(crate) item: ItemId,
    /// The impl item the path belongs to.
    pub(crate) impl_item: Option<ItemId>,
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
#[derive(Debug, Clone, Copy, Spanned)]
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
#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone, Copy, Spanned)]
#[non_exhaustive]
pub(crate) struct ItemFn<'hir> {
    /// The span of the function.
    #[rune(span)]
    pub(crate) span: Span,
    /// The arguments of the function.
    pub(crate) args: &'hir [FnArg<'hir>],
    /// The body of the function.
    pub(crate) body: &'hir Block<'hir>,
}

/// A single argument to a function.
#[derive(Debug, Clone, Copy, Spanned)]
#[non_exhaustive]
pub(crate) enum FnArg<'hir> {
    /// The `self` parameter.
    SelfValue(Span),
    /// Function argument is a pattern binding.
    Pat(&'hir Pat<'hir>),
}

/// A block of statements.
#[derive(Debug, Clone, Copy, Spanned)]
#[non_exhaustive]
pub(crate) struct Block<'hir> {
    /// The span of the block.
    #[rune(span)]
    pub(crate) span: Span,
    /// Statements in the block.
    pub(crate) statements: &'hir [Stmt<'hir>],
    /// Variables that need to be dropped by the end of this block.
    #[allow(unused)]
    pub(crate) drop: &'hir [Name<'hir>],
}

impl Block<'_> {
    /// Test if the block doesn't produce anything. Which is when the last
    /// element is either a non-expression or is an expression terminated by a
    /// semi.
    pub(crate) fn produces_nothing(&self) -> bool {
        matches!(self.statements.last(), Some(Stmt::Semi(..)) | None)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AsyncBlock<'hir> {
    pub(crate) block: &'hir Block<'hir>,
    pub(crate) captures: &'hir [Name<'hir>],
}

/// A statement within a block.
#[derive(Debug, Clone, Copy, Spanned)]
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
#[derive(Debug, Clone, Copy, Spanned)]
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
