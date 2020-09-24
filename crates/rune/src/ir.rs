//! Intermediate representation of Rune that can be evaluated in constant
//! contexts.
//!
//! This is part of the [Rune Language].
//! [Rune Language]: https://rune-rs.github.io

use crate::Spanned;
use runestick::{ConstValue, Span};

macro_rules! decl_kind {
    (
        $(#[$meta:meta])*
        pub enum $name:ident {
            $($(#[$field_meta:meta])* $variant:ident($ty:ty)),* $(,)?
        }
    ) => {
        $(#[$meta])*
        pub enum $name {
            $($(#[$field_meta])* $variant($ty),)*
        }

        $(
            impl From<$ty> for $name {
                fn from(value: $ty) -> $name {
                    $name::$variant(value)
                }
            }
        )*
    }
}

/// A single operation in the Rune intermediate language.
#[derive(Debug, Clone, Spanned)]
pub struct Ir {
    #[rune(span)]
    pub span: Span,
    pub kind: IrKind,
}

impl Ir {
    /// Construct a new intermediate instruction.
    pub fn new<S, K>(spanned: S, kind: K) -> Self
    where
        S: Spanned,
        IrKind: From<K>,
    {
        Self {
            span: spanned.span(),
            kind: IrKind::from(kind),
        }
    }
}

decl_kind! {
    /// The kind of an intermediate operation.
    #[derive(Debug, Clone)]
    pub enum IrKind {
        /// Push a scope with the given instructions.
        Scope(IrScope),
        /// A binary operation.
        Binary(IrBinary),
        /// Declare a local variable with the value of the operand.
        Decl(IrDecl),
        /// Update a local variable with the value of the operand.
        Set(IrSet),
        /// A template.
        Template(IrTemplate),
        /// A local name. Could either be a local variable or a reference to
        /// something else, like another const declaration.
        Name(Box<str>),
        /// A constant value.
        Value(ConstValue),
        /// A sequence of conditional branches.
        Branches(IrBranches),
        /// A loop.
        Loop(IrLoop),
        /// A break to the given target.
        Break(IrBreak),
        /// Constructing a vector.
        Vec(IrVec),
        /// Constructing a tuple.
        Tuple(IrTuple),
        /// Constructing an object.
        Object(IrObject),
    }
}

/// Definition of a new variable scope.
#[derive(Debug, Clone, Spanned)]
pub struct IrScope {
    /// The span of the scope.
    #[rune(span)]
    pub span: Span,
    /// Instructions in the scope.
    pub instructions: Vec<Ir>,
    /// The implicit value of the scope.
    pub last: Option<Box<Ir>>,
}

/// A binary operation.
#[derive(Debug, Clone, Spanned)]
pub struct IrBinary {
    /// The span of the binary op.
    #[rune(span)]
    pub span: Span,
    /// The binary operation.
    pub op: IrBinaryOp,
    /// The left-hand side of the binary op.
    pub lhs: Box<Ir>,
    /// The right-hand side of the binary op.
    pub rhs: Box<Ir>,
}

/// A local variable declaration.
#[derive(Debug, Clone, Spanned)]
pub struct IrDecl {
    /// The span of the declaration.
    #[rune(span)]
    pub span: Span,
    /// The name of the variable.
    pub name: Box<str>,
    /// The value of the variable.
    pub value: Box<Ir>,
}

/// Update a local variable.
#[derive(Debug, Clone, Spanned)]
pub struct IrSet {
    /// The span of the set operation.
    #[rune(span)]
    pub span: Span,
    /// The name of the local variable to set.
    pub name: Box<str>,
    /// The value of the variable.
    pub value: Box<Ir>,
}

/// A string template.
#[derive(Debug, Clone, Spanned)]
pub struct IrTemplate {
    /// The span of the template.
    #[rune(span)]
    pub span: Span,
    /// Template components.
    pub components: Vec<IrTemplateComponent>,
}

/// A string template.
#[derive(Debug, Clone)]
pub enum IrTemplateComponent {
    /// An ir expression.
    Ir(Ir),
    /// A literal string.
    String(Box<str>),
}

/// Branch conditions in intermediate representation.
#[derive(Debug, Clone)]
pub struct IrBranches {
    /// branches and their associated conditions.
    pub branches: Vec<(Ir, IrScope)>,
    /// The default fallback branch.
    pub default_branch: Option<IrScope>,
}

/// A loop with an optional condition.
#[derive(Debug, Clone, Spanned)]
pub struct IrLoop {
    /// The span of the loop.
    #[rune(span)]
    pub span: Span,
    /// The label of the loop.
    pub label: Option<Box<str>>,
    /// The condition of the loop.
    pub condition: Option<Box<Ir>>,
    /// The body of the loop.
    pub body: IrScope,
}

/// A break operation.
#[derive(Debug, Clone, Spanned)]
pub struct IrBreak {
    /// The span of the break.
    #[rune(span)]
    pub span: Span,
    /// The kind of the break.
    pub kind: IrBreakKind,
}

/// The kind of a break expression.
#[derive(Debug, Clone)]
pub enum IrBreakKind {
    /// Break to the next loop.
    Inherent,
    /// Break to the given label.
    Label(Box<str>),
    /// Break with the value acquired from evaluating the ir.
    Ir(Box<Ir>),
}

/// Tuple expression.
#[derive(Debug, Clone, Spanned)]
pub struct IrTuple {
    /// Span of the tuple.
    #[rune(span)]
    pub span: Span,
    /// Arguments to construct the tuple.
    pub items: Box<[Ir]>,
}

/// Object expression.
#[derive(Debug, Clone, Spanned)]
pub struct IrObject {
    /// Span of the object.
    #[rune(span)]
    pub span: Span,
    /// Field initializations.
    pub assignments: Box<[(Box<str>, Ir)]>,
}

/// Vector expression.
#[derive(Debug, Clone, Spanned)]
pub struct IrVec {
    /// Span of the vector.
    #[rune(span)]
    pub span: Span,
    /// Arguments to construct the vector.
    pub items: Box<[Ir]>,
}

/// A binary operation.
#[derive(Debug, Clone, Copy)]
pub enum IrBinaryOp {
    /// Add `+`.
    Add,
    /// Subtract `-`.
    Sub,
    /// Multiplication `*`.
    Mul,
    /// Division `/`.
    Div,
    /// `<`,
    Lt,
    /// `<=`,
    Lte,
    /// `==`,
    Eq,
    /// `>`,
    Gt,
    /// `>=`,
    Gte,
    /// `<<`.
    Shl,
    /// `>>`.
    Shr,
}
