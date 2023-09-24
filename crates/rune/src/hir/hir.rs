use core::fmt;
use core::num::NonZeroUsize;

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::{self, String};
use crate::ast::{self, Span, Spanned};
use crate::compile::{ItemId, ModId};
use crate::parse::NonZeroId;
use crate::runtime::{format, Type, TypeCheck};
use crate::Hash;

/// An owned name.
#[derive(Debug, TryClone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum OwnedName {
    SelfValue,
    Str(String),
    Id(usize),
}

impl OwnedName {
    /// Get name as reference.
    pub(crate) fn as_ref(&self) -> Name<'_> {
        match self {
            OwnedName::SelfValue => Name::SelfValue,
            OwnedName::Str(name) => Name::Str(name),
            OwnedName::Id(id) => Name::Id(*id),
        }
    }
}

impl fmt::Display for OwnedName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OwnedName::SelfValue => "self".fmt(f),
            OwnedName::Str(name) => name.fmt(f),
            OwnedName::Id(id) => id.fmt(f),
        }
    }
}

/// A captured variable.
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[try_clone(copy)]
pub(crate) enum Name<'hir> {
    /// Capture of the `self` value.
    SelfValue,
    /// Capture of a named variable.
    Str(&'hir str),
    /// Anonymous variable.
    Id(usize),
}

impl<'hir> Name<'hir> {
    /// Coerce into an owned name.
    pub(crate) fn into_owned(self) -> alloc::Result<OwnedName> {
        Ok(match self {
            Name::SelfValue => OwnedName::SelfValue,
            Name::Str(name) => OwnedName::Str(name.try_to_owned()?),
            Name::Id(id) => OwnedName::Id(id),
        })
    }

    /// Test if the name starts with the given test.
    pub(crate) fn starts_with(&self, test: fn(char) -> bool) -> bool {
        let Name::Str(name) = self else {
            return false;
        };

        name.starts_with(test)
    }
}

impl fmt::Display for Name<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Name::SelfValue => "self".fmt(f),
            Name::Str(name) => name.fmt(f),
            Name::Id(id) => id.fmt(f),
        }
    }
}

/// A pattern.
#[derive(Debug, TryClone, Clone, Copy, Spanned)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct Pat<'hir> {
    /// The span of the pattern.
    #[rune(span)]
    pub(crate) span: Span,
    /// The kind of the pattern.
    pub(crate) kind: PatKind<'hir>,
}

#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
pub(crate) enum PatPathKind<'hir> {
    Kind(&'hir PatSequenceKind),
    Ident(&'hir str),
}

/// The kind of a [Pat].
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
pub(crate) enum PatKind<'hir> {
    /// An ignored binding.
    Ignore,
    /// A path pattern.
    Path(&'hir PatPathKind<'hir>),
    /// A literal pattern. This is represented as an expression.
    Lit(&'hir Expr<'hir>),
    /// A tuple pattern.
    Sequence(&'hir PatSequence<'hir>),
    /// An object pattern.
    Object(&'hir PatObject<'hir>),
}

#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
pub(crate) enum PatSequenceKind {
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
        type_check: TypeCheck,
        count: usize,
        is_open: bool,
    },
}

/// Items pattern matching.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct PatSequence<'hir> {
    /// The kind of pattern items.
    pub(crate) kind: PatSequenceKind,
    /// The items in the tuple.
    pub(crate) items: &'hir [Pat<'hir>],
}

/// Object pattern matching.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct PatObject<'hir> {
    /// The kind of pattern items.
    pub(crate) kind: PatSequenceKind,
    /// Bindings associated with the pattern.
    pub(crate) bindings: &'hir [Binding<'hir>],
}

#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
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
#[derive(Debug, TryClone, Clone, Copy, Spanned)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct Expr<'hir> {
    /// Span of the expression.
    #[rune(span)]
    pub(crate) span: Span,
    /// The kind of the expression.
    pub(crate) kind: ExprKind<'hir>,
}

/// The kind of a number.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
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
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) enum ExprKind<'hir> {
    Variable(Name<'hir>),
    Type(Type),
    Fn(Hash),
    Path,
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
    Break(&'hir ExprBreak<'hir>),
    Continue(&'hir ExprContinue<'hir>),
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
#[derive(Debug, TryClone, Clone, Copy, Spanned)]
#[try_clone(copy)]
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
#[derive(Debug, TryClone, Clone, Copy, Spanned)]
#[try_clone(copy)]
pub(crate) struct BuiltInFormat<'hir> {
    #[rune(span)]
    pub(crate) span: Span,
    /// The fill character to use.
    pub(crate) fill: Option<char>,
    /// Alignment specification.
    pub(crate) align: Option<format::Alignment>,
    /// Width to fill.
    pub(crate) width: Option<NonZeroUsize>,
    /// Precision to fill.
    pub(crate) precision: Option<NonZeroUsize>,
    /// A specification of flags.
    pub(crate) flags: Option<format::Flags>,
    /// The format specification type.
    pub(crate) format_type: Option<format::Type>,
    /// The value being formatted.
    pub(crate) value: Expr<'hir>,
}

/// An assign expression `a = b`.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct ExprAssign<'hir> {
    /// The expression being assigned to.
    pub(crate) lhs: Expr<'hir>,
    /// The value.
    pub(crate) rhs: Expr<'hir>,
}

/// A `loop` expression: `loop { ... }`.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct ExprLoop<'hir> {
    /// A label.
    pub(crate) label: Option<&'hir str>,
    /// A condition to execute the loop, if a condition is necessary.
    pub(crate) condition: Option<&'hir Condition<'hir>>,
    /// The body of the loop.
    pub(crate) body: Block<'hir>,
    /// Variables that have been defined by the loop header.
    #[allow(unused)]
    pub(crate) drop: &'hir [Name<'hir>],
}

/// A `for` loop over an iterator: `for i in [1, 2, 3] {}`.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct ExprFor<'hir> {
    /// The label of the loop.
    pub(crate) label: Option<&'hir str>,
    /// The pattern binding to use.
    /// Non-trivial pattern bindings will panic if the value doesn't match.
    pub(crate) binding: Pat<'hir>,
    /// Expression producing the iterator.
    pub(crate) iter: Expr<'hir>,
    /// The body of the loop.
    pub(crate) body: Block<'hir>,
    /// Variables that have been defined by the loop header.
    #[allow(unused)]
    pub(crate) drop: &'hir [Name<'hir>],
}

/// A let expression `let <name> = <expr>`
#[derive(Debug, TryClone, Clone, Copy, Spanned)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct ExprLet<'hir> {
    /// The name of the binding.
    pub(crate) pat: Pat<'hir>,
    /// The expression the binding is assigned to.
    pub(crate) expr: Expr<'hir>,
}

/// A sequence of conditional branches.
///
/// This is lower from if statements, such as:
///
/// ```text
/// if cond { true } else { false }
/// ```
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct Conditional<'hir> {
    /// Else if branches.
    pub(crate) branches: &'hir [ConditionalBranch<'hir>],
}

/// An else branch of an if expression.
#[derive(Debug, TryClone, Clone, Copy, Spanned)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct ConditionalBranch<'hir> {
    /// Span of the expression.
    #[rune(span)]
    pub(crate) span: Span,
    /// The condition for the branch. Empty condition means that this is the
    /// fallback branch.
    pub(crate) condition: Option<&'hir Condition<'hir>>,
    /// The body of the else statement.
    pub(crate) block: Block<'hir>,
    /// Variables that have been defined by the conditional header.
    #[allow(unused)]
    pub(crate) drop: &'hir [Name<'hir>],
}

/// A match expression.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct ExprMatch<'hir> {
    /// The expression who's result we match over.
    pub(crate) expr: Expr<'hir>,
    /// Branches.
    pub(crate) branches: &'hir [ExprMatchBranch<'hir>],
}

/// A match branch.
#[derive(Debug, TryClone, Clone, Copy, Spanned)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct ExprMatchBranch<'hir> {
    /// Span of the expression.
    #[rune(span)]
    pub(crate) span: Span,
    /// The pattern to match.
    pub(crate) pat: Pat<'hir>,
    /// The branch condition.
    pub(crate) condition: Option<&'hir Expr<'hir>>,
    /// The body of the match.
    pub(crate) body: Expr<'hir>,
    /// Variables that have been defined by this match branch, which needs to be
    /// dropped.
    #[allow(unused)]
    pub(crate) drop: &'hir [Name<'hir>],
}

#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
pub(crate) enum Call<'hir> {
    Var {
        /// The name of the variable being called.
        name: Name<'hir>,
    },
    Associated {
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
        /// The module the constant function is being called from.
        from_module: ModId,
        /// The item the constant function is being called from.
        from_item: ItemId,
        /// The identifier of the constant function.
        id: NonZeroId,
    },
}

/// A function call `<expr>(<args>)`.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct ExprCall<'hir> {
    /// The call being performed.
    pub(crate) call: Call<'hir>,
    /// The arguments of the function call.
    pub(crate) args: &'hir [Expr<'hir>],
}

/// A field access `<expr>.<field>`.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct ExprFieldAccess<'hir> {
    /// The expr where the field is being accessed.
    pub(crate) expr: Expr<'hir>,
    /// The field being accessed.
    pub(crate) expr_field: ExprField<'hir>,
}

/// The field being accessed.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) enum ExprField<'hir> {
    /// A tuple index.
    ///
    /// ```text
    /// 1
    /// ```
    Index(usize),
    /// A field identifier.
    ///
    /// ```text
    /// field
    /// ```
    Ident(&'hir str),
    /// A field identifier immediately followed by generic expressions.
    ///
    /// ```text
    /// field<1, string>
    /// ```
    IdentGenerics(&'hir str, Hash),
}

/// A binary expression.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct ExprBinary<'hir> {
    /// The left-hand side of a binary operation.
    pub(crate) lhs: Expr<'hir>,
    /// The operator.
    pub(crate) op: ast::BinOp,
    /// The right-hand side of a binary operation.
    pub(crate) rhs: Expr<'hir>,
}

/// A unary expression.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct ExprUnary<'hir> {
    /// The operation to apply.
    pub(crate) op: ast::UnOp,
    /// The expression of the operation.
    pub(crate) expr: Expr<'hir>,
}

/// An index get operation `<t>[<index>]`.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct ExprIndex<'hir> {
    /// The target of the index set.
    pub(crate) target: Expr<'hir>,
    /// The indexing expression.
    pub(crate) index: Expr<'hir>,
}

/// An async block being called.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct ExprAsyncBlock<'hir> {
    pub(crate) hash: Hash,
    pub(crate) do_move: bool,
    pub(crate) captures: &'hir [Name<'hir>],
}

#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
pub(crate) struct ExprBreak<'hir> {
    /// Label being continued.
    pub(crate) label: Option<&'hir str>,
    /// Value being broken with.
    pub(crate) expr: Option<&'hir Expr<'hir>>,
    /// Variables that goes out of scope.
    #[allow(unused)]
    pub(crate) drop: &'hir [Name<'hir>],
}

#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
pub(crate) struct ExprContinue<'hir> {
    /// Label being continued.
    pub(crate) label: Option<&'hir str>,
    /// Variables that goes out of scope.
    #[allow(unused)]
    pub(crate) drop: &'hir [Name<'hir>],
}

/// A `select` expression that selects over a collection of futures.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct ExprSelect<'hir> {
    /// The branches of the select.
    pub(crate) branches: &'hir [ExprSelectBranch<'hir>],
}

/// A single selection branch.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) enum ExprSelectBranch<'hir> {
    /// A patterned branch.
    Pat(&'hir ExprSelectPatBranch<'hir>),
    /// A default branch.
    Default(&'hir Expr<'hir>),
}

/// A single selection branch.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct ExprSelectPatBranch<'hir> {
    /// The identifier to bind the result to.
    pub(crate) pat: Pat<'hir>,
    /// The expression that should evaluate to a future.
    pub(crate) expr: Expr<'hir>,
    /// The body of the expression.
    pub(crate) body: Expr<'hir>,
    /// Variables that need to be dropped by the end of this block.
    #[allow(unused)]
    pub(crate) drop: &'hir [Name<'hir>],
}

/// Calling a closure.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct ExprCallClosure<'hir> {
    pub(crate) do_move: bool,
    pub(crate) hash: Hash,
    pub(crate) captures: &'hir [Name<'hir>],
}

/// A closure expression.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct ExprClosure<'hir> {
    /// Arguments to the closure.
    pub(crate) args: &'hir [FnArg<'hir>],
    /// The body of the closure.
    pub(crate) body: Expr<'hir>,
    /// Captures in the closure.
    pub(crate) captures: &'hir [Name<'hir>],
}

#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
pub(crate) enum ExprObjectKind {
    EmptyStruct { hash: Hash },
    Struct { hash: Hash },
    StructVariant { hash: Hash },
    ExternalType { hash: Hash, args: usize },
    Anonymous,
}

/// An object expression.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct ExprObject<'hir> {
    /// The kind of an object being created.
    pub(crate) kind: ExprObjectKind,
    /// Assignments in the object.
    pub(crate) assignments: &'hir [FieldAssign<'hir>],
}

/// A single field assignment in an object expression.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct FieldAssign<'hir> {
    /// The key of the field.
    pub(crate) key: (Span, &'hir str),
    /// The assigned expression of the field.
    pub(crate) assign: Expr<'hir>,
    /// The position of the field in its containing type declaration.
    pub(crate) position: Option<usize>,
}

/// A literal vector.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct ExprSeq<'hir> {
    /// Items in the vector.
    pub(crate) items: &'hir [Expr<'hir>],
}

/// A range expression such as `a .. b` or `a ..= b`.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) enum ExprRange<'hir> {
    /// `start..`.
    RangeFrom { start: Expr<'hir> },
    /// `..`.
    RangeFull,
    /// `start..=end`.
    RangeInclusive { start: Expr<'hir>, end: Expr<'hir> },
    /// `..=end`.
    RangeToInclusive { end: Expr<'hir> },
    /// `..end`.
    RangeTo { end: Expr<'hir> },
    /// `start..end`.
    Range { start: Expr<'hir>, end: Expr<'hir> },
}

/// The condition in an if statement.
#[derive(Debug, TryClone, Clone, Copy, Spanned)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) enum Condition<'hir> {
    /// A regular expression.
    Expr(&'hir Expr<'hir>),
    /// A pattern match.
    ExprLet(&'hir ExprLet<'hir>),
}

#[derive(Debug, TryClone, Clone, Copy, Spanned)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct ItemFn<'hir> {
    /// The span of the function.
    #[rune(span)]
    pub(crate) span: Span,
    /// The arguments of the function.
    pub(crate) args: &'hir [FnArg<'hir>],
    /// The body of the function.
    pub(crate) body: Block<'hir>,
}

/// A single argument to a function.
#[derive(Debug, TryClone, Clone, Copy, Spanned)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) enum FnArg<'hir> {
    /// The `self` parameter.
    SelfValue(Span),
    /// Function argument is a pattern binding.
    Pat(&'hir Pat<'hir>),
}

/// A block of statements.
#[derive(Debug, TryClone, Clone, Copy, Spanned)]
#[try_clone(copy)]
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

#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
pub(crate) struct AsyncBlock<'hir> {
    pub(crate) block: Block<'hir>,
    pub(crate) captures: &'hir [Name<'hir>],
}

/// A statement within a block.
#[derive(Debug, TryClone, Clone, Copy, Spanned)]
#[try_clone(copy)]
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
#[derive(Debug, TryClone, Clone, Copy, Spanned)]
#[try_clone(copy)]
#[non_exhaustive]
pub(crate) struct Local<'hir> {
    /// The span of the local declaration.
    #[rune(span)]
    pub(crate) span: Span,
    /// The name of the binding.
    pub(crate) pat: Pat<'hir>,
    /// The expression the binding is assigned to.
    pub(crate) expr: Expr<'hir>,
}
