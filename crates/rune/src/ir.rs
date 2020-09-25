//! Intermediate representation of Rune that can be evaluated in constant
//! contexts.
//!
//! This is part of the [Rune Language].
//! [Rune Language]: https://rune-rs.github.io

use crate::ir_value::IrValue;
use crate::{IrError, Spanned};
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

/// The target of a set operation.
#[derive(Debug, Clone, Spanned)]
pub struct IrTarget {
    /// Span of the target.
    #[rune(span)]
    pub span: Span,
    /// Kind of the target.
    pub kind: IrTargetKind,
}

/// The kind of the target.
#[derive(Debug, Clone)]
pub enum IrTargetKind {
    /// A variable.
    Name(Box<str>),
    /// A field target.
    Field(Box<IrTarget>, Box<str>),
    /// An index target.
    Index(Box<IrTarget>, usize),
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
        /// Set the given target.
        Set(IrSet),
        /// Assign the given target.
        Assign(IrAssign),
        /// A template.
        Template(IrTemplate),
        /// A named value.
        Name(Box<str>),
        /// A local name. Could either be a local variable or a reference to
        /// something else, like another const declaration.
        Target(IrTarget),
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

/// Set a target.
#[derive(Debug, Clone, Spanned)]
pub struct IrSet {
    /// The span of the set operation.
    #[rune(span)]
    pub span: Span,
    /// The target to set.
    pub target: IrTarget,
    /// The value to set the target.
    pub value: Box<Ir>,
}

/// Assign a target.
#[derive(Debug, Clone, Spanned)]
pub struct IrAssign {
    /// The span of the set operation.
    #[rune(span)]
    pub span: Span,
    /// The name of the target to assign.
    pub target: IrTarget,
    /// The value to assign.
    pub value: Box<Ir>,
    /// The assign operation.
    pub op: IrAssignOp,
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
    pub branches: Vec<(IrCondition, IrScope)>,
    /// The default fallback branch.
    pub default_branch: Option<IrScope>,
}

/// The condition for a branch.
#[derive(Debug, Clone, Spanned)]
pub enum IrCondition {
    /// A simple conditiona ir expression.
    Ir(Ir),
    /// A pattern match.
    Let(IrLet),
}

/// A pattern match.
#[derive(Debug, Clone, Spanned)]
pub struct IrLet {
    /// The span of the let condition.
    #[rune(span)]
    pub span: Span,
    /// The pattern.
    pub pat: IrPat,
    /// The expression the pattern is evaluated on.
    pub ir: Ir,
}

/// A pattern.
#[derive(Debug, Clone)]
pub enum IrPat {
    /// An ignore pattern `_`.
    Ignore,
    /// A named binding.
    Binding(Box<str>),
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
    pub condition: Option<Box<IrCondition>>,
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
    /// `<<`.
    Shl,
    /// `>>`.
    Shr,
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
}

/// An assign operation.
#[derive(Debug, Clone, Copy)]
pub enum IrAssignOp {
    /// `+=`.
    Add,
    /// `-=`.
    Sub,
    /// `*=`.
    Mul,
    /// `/=`.
    Div,
    /// `<<=`.
    Shl,
    /// `>>=`.
    Shr,
}

impl IrAssignOp {
    /// Perform the given assign operation.
    pub(crate) fn assign<S>(
        self,
        spanned: S,
        target: &mut IrValue,
        operand: IrValue,
    ) -> Result<(), IrError>
    where
        S: Copy + Spanned,
    {
        match (target, operand) {
            (IrValue::Integer(target), IrValue::Integer(operand)) => {
                self.assign_int(spanned, target, operand)?;
            }
            _ => return Err(IrError::custom(spanned, "unsupported operands")),
        }

        Ok(())
    }

    /// Perform the given assign operation.
    pub(crate) fn assign_int<S>(
        self,
        spanned: S,
        target: &mut i64,
        operand: i64,
    ) -> Result<(), IrError>
    where
        S: Copy + Spanned,
    {
        use std::convert::TryFrom;

        match self {
            IrAssignOp::Add => {
                *target = target
                    .checked_add(operand)
                    .ok_or_else(|| IrError::custom(spanned, "integer overflow"))?;
            }
            IrAssignOp::Sub => {
                *target = target
                    .checked_sub(operand)
                    .ok_or_else(|| IrError::custom(spanned, "integer underflow"))?;
            }
            IrAssignOp::Mul => {
                *target = target
                    .checked_mul(operand)
                    .ok_or_else(|| IrError::custom(spanned, "integer overflow"))?;
            }
            IrAssignOp::Div => {
                *target = target
                    .checked_div(operand)
                    .ok_or_else(|| IrError::custom(spanned, "division by zero"))?;
            }
            IrAssignOp::Shl => {
                let operand =
                    u32::try_from(operand).map_err(|_| IrError::custom(spanned, "bad operand"))?;

                *target = target
                    .checked_shl(operand)
                    .ok_or_else(|| IrError::custom(spanned, "integer shift overflow"))?;
            }
            IrAssignOp::Shr => {
                let operand =
                    u32::try_from(operand).map_err(|_| IrError::custom(spanned, "bad operand"))?;

                *target = target
                    .checked_shr(operand)
                    .ok_or_else(|| IrError::custom(spanned, "integer shift underflow"))?;
            }
        }

        Ok(())
    }
}
