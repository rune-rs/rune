//! Intermediate representation of Rune that can be evaluated in constant
//! contexts.
//!
//! This is part of the [Rune Language](https://rune-rs.github.io).

mod eval;
mod ir_compiler;
mod ir_error;
mod ir_interpreter;
mod ir_query;
mod ir_value;

pub use self::eval::{IrEval, IrEvalBreak, IrEvalOutcome};
pub use self::ir_compiler::{IrCompile, IrCompiler};
pub use self::ir_error::{IrError, IrErrorKind};
pub use self::ir_interpreter::IrInterpreter;
pub use self::ir_value::IrValue;

pub(crate) use self::ir_interpreter::IrBudget;
pub(crate) use self::ir_query::IrQuery;

use crate::Spanned;
use runestick::{ConstValue, Span};

macro_rules! decl_kind {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $($(#[$field_meta:meta])* $variant:ident($ty:ty)),* $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis enum $name {
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
    pub(crate) span: Span,
    pub(crate) kind: IrKind,
}

impl Ir {
    /// Construct a new intermediate instruction.
    pub(crate) fn new<S, K>(spanned: S, kind: K) -> Self
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
    pub(crate) span: Span,
    /// Kind of the target.
    pub(crate) kind: IrTargetKind,
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
        /// A call.
        Call(IrCall),
    }
}

/// An interpeted function.
#[derive(Debug, Clone, Spanned)]
pub struct IrFn {
    /// The span of the function.
    #[rune(span)]
    pub(crate) span: Span,
    /// The number of arguments the function takes and their names.
    pub(crate) args: Vec<Box<str>>,
    /// The scope for the function.
    pub(crate) ir: Ir,
}

/// Definition of a new variable scope.
#[derive(Debug, Clone, Spanned)]
pub struct IrScope {
    /// The span of the scope.
    #[rune(span)]
    pub(crate) span: Span,
    /// Instructions in the scope.
    pub(crate) instructions: Vec<Ir>,
    /// The implicit value of the scope.
    pub(crate) last: Option<Box<Ir>>,
}

/// A binary operation.
#[derive(Debug, Clone, Spanned)]
pub struct IrBinary {
    /// The span of the binary op.
    #[rune(span)]
    pub(crate) span: Span,
    /// The binary operation.
    pub(crate) op: IrBinaryOp,
    /// The left-hand side of the binary op.
    pub(crate) lhs: Box<Ir>,
    /// The right-hand side of the binary op.
    pub(crate) rhs: Box<Ir>,
}

/// A local variable declaration.
#[derive(Debug, Clone, Spanned)]
pub struct IrDecl {
    /// The span of the declaration.
    #[rune(span)]
    pub(crate) span: Span,
    /// The name of the variable.
    pub(crate) name: Box<str>,
    /// The value of the variable.
    pub(crate) value: Box<Ir>,
}

/// Set a target.
#[derive(Debug, Clone, Spanned)]
pub struct IrSet {
    /// The span of the set operation.
    #[rune(span)]
    pub(crate) span: Span,
    /// The target to set.
    pub(crate) target: IrTarget,
    /// The value to set the target.
    pub(crate) value: Box<Ir>,
}

/// Assign a target.
#[derive(Debug, Clone, Spanned)]
pub struct IrAssign {
    /// The span of the set operation.
    #[rune(span)]
    pub(crate) span: Span,
    /// The name of the target to assign.
    pub(crate) target: IrTarget,
    /// The value to assign.
    pub(crate) value: Box<Ir>,
    /// The assign operation.
    pub(crate) op: IrAssignOp,
}

/// A string template.
#[derive(Debug, Clone, Spanned)]
pub struct IrTemplate {
    /// The span of the template.
    #[rune(span)]
    pub(crate) span: Span,
    /// Template components.
    pub(crate) components: Vec<IrTemplateComponent>,
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
    pub(crate) branches: Vec<(IrCondition, IrScope)>,
    /// The default fallback branch.
    pub(crate) default_branch: Option<IrScope>,
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
    pub(crate) span: Span,
    /// The pattern.
    pub(crate) pat: IrPat,
    /// The expression the pattern is evaluated on.
    pub(crate) ir: Ir,
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
    pub(crate) span: Span,
    /// The label of the loop.
    pub(crate) label: Option<Box<str>>,
    /// The condition of the loop.
    pub(crate) condition: Option<Box<IrCondition>>,
    /// The body of the loop.
    pub(crate) body: IrScope,
}

/// A break operation.
#[derive(Debug, Clone, Spanned)]
pub struct IrBreak {
    /// The span of the break.
    #[rune(span)]
    pub(crate) span: Span,
    /// The kind of the break.
    pub(crate) kind: IrBreakKind,
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
    pub(crate) span: Span,
    /// Arguments to construct the tuple.
    pub(crate) items: Box<[Ir]>,
}

/// Object expression.
#[derive(Debug, Clone, Spanned)]
pub struct IrObject {
    /// Span of the object.
    #[rune(span)]
    pub(crate) span: Span,
    /// Field initializations.
    pub(crate) assignments: Box<[(Box<str>, Ir)]>,
}

/// Call expressions.
#[derive(Debug, Clone, Spanned)]
pub struct IrCall {
    /// Span of the call.
    #[rune(span)]
    pub(crate) span: Span,
    /// The target of the call.
    pub(crate) target: Box<str>,
    /// Arguments to the call.
    pub(crate) args: Vec<Ir>,
}

/// Vector expression.
#[derive(Debug, Clone, Spanned)]
pub struct IrVec {
    /// Span of the vector.
    #[rune(span)]
    pub(crate) span: Span,
    /// Arguments to construct the vector.
    pub(crate) items: Box<[Ir]>,
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
        if let IrValue::Integer(target) = target {
            if let IrValue::Integer(operand) = operand {
                return self.assign_int(spanned, target, operand);
            }
        }

        Err(IrError::msg(spanned, "unsupported operands"))
    }

    /// Perform the given assign operation.
    fn assign_int<S>(
        self,
        spanned: S,
        target: &mut num::BigInt,
        operand: num::BigInt,
    ) -> Result<(), IrError>
    where
        S: Copy + Spanned,
    {
        use std::convert::TryFrom;
        use std::ops::{AddAssign, MulAssign, ShlAssign, ShrAssign, SubAssign};

        match self {
            IrAssignOp::Add => {
                target.add_assign(operand);
            }
            IrAssignOp::Sub => {
                target.sub_assign(operand);
            }
            IrAssignOp::Mul => {
                target.mul_assign(operand);
            }
            IrAssignOp::Div => {
                *target = target
                    .checked_div(&operand)
                    .ok_or_else(|| IrError::msg(spanned, "division by zero"))?;
            }
            IrAssignOp::Shl => {
                let operand =
                    u32::try_from(operand).map_err(|_| IrError::msg(spanned, "bad operand"))?;

                target.shl_assign(operand);
            }
            IrAssignOp::Shr => {
                let operand =
                    u32::try_from(operand).map_err(|_| IrError::msg(spanned, "bad operand"))?;

                target.shr_assign(operand);
            }
        }

        Ok(())
    }
}
