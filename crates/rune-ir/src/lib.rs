//! <div align="center">
//!     <img alt="Rune Logo" src="https://raw.githubusercontent.com/rune-rs/rune/master/assets/icon.png" />
//! </div>
//!
//! <br>
//!
//! <div align="center">
//! <a href="https://rune-rs.github.io">
//!     <b>Visit the site 🌐</b>
//! </a>
//! -
//! <a href="https://rune-rs.github.io/book/">
//!     <b>Read the book 📖</b>
//! </a>
//! </div>
//!
//! <br>
//!
//! <div align="center">
//! <a href="https://github.com/rune-rs/rune/actions">
//!     <img alt="Build Status" src="https://github.com/rune-rs/rune/workflows/Build/badge.svg">
//! </a>
//!
//! <a href="https://github.com/rune-rs/rune/actions">
//!     <img alt="Site Status" src="https://github.com/rune-rs/rune/workflows/Site/badge.svg">
//! </a>
//!
//! <a href="https://crates.io/crates/rune">
//!     <img alt="crates.io" src="https://img.shields.io/crates/v/rune.svg">
//! </a>
//!
//! <a href="https://docs.rs/rune">
//!     <img alt="docs.rs" src="https://docs.rs/rune/badge.svg">
//! </a>
//!
//! <a href="https://discord.gg/v5AeNkT">
//!     <img alt="Chat on Discord" src="https://img.shields.io/discord/558644981137670144.svg?logo=discord&style=flat-square">
//! </a>
//! </div>
//!
//! Intermediate representation of Rune that can be evaluated in constant
//! contexts.
//!
//! This is part of the [Rune Language].
//! [Rune Language]: https://rune-rs.github.io

use runestick::{ConstValue, Span};

/// A single operation in the Rune intermediate language.
#[derive(Debug, Clone)]
pub struct Ir {
    pub span: Span,
    pub kind: IrKind,
}

impl Ir {
    /// Construct a new intermediate instruction.
    pub fn new<K>(span: Span, kind: K) -> Self
    where
        IrKind: From<K>,
    {
        Self {
            span,
            kind: IrKind::from(kind),
        }
    }
}

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
    }
}

/// Definition of a new variable scope.
#[derive(Debug, Clone)]
pub struct IrScope {
    /// The span of the scope.
    pub span: Span,
    /// Instructions in the scope.
    pub instructions: Vec<Ir>,
    /// The implicit value of the scope.
    pub last: Option<Box<Ir>>,
}

/// A binary operation.
#[derive(Debug, Clone)]
pub struct IrBinary {
    /// The span of the binary op.
    pub span: Span,
    /// The binary operation.
    pub op: IrBinaryOp,
    /// The left-hand side of the binary op.
    pub lhs: Box<Ir>,
    /// The right-hand side of the binary op.
    pub rhs: Box<Ir>,
}

/// A local variable declaration.
#[derive(Debug, Clone)]
pub struct IrDecl {
    /// The span of the declaration.
    pub span: Span,
    /// The name of the variable.
    pub name: Box<str>,
    /// The value of the variable.
    pub value: Box<Ir>,
}

/// Update a local variable.
#[derive(Debug, Clone)]
pub struct IrSet {
    /// The span of the set operation.
    pub span: Span,
    /// The name of the local variable to set.
    pub name: Box<str>,
    /// The value of the variable.
    pub value: Box<Ir>,
}

/// A string template.
#[derive(Debug, Clone)]
pub struct IrTemplate {
    /// The span of the template.
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
#[derive(Debug, Clone)]
pub struct IrLoop {
    /// The span of the loop.
    pub span: Span,
    /// The label of the loop.
    pub label: Option<Box<str>>,
    /// The condition of the loop.
    pub condition: Option<Box<Ir>>,
    /// The body of the loop.
    pub body: IrScope,
}

/// A break operation.
#[derive(Debug, Clone)]
pub struct IrBreak {
    /// The span of the break.
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
#[derive(Debug, Clone)]
pub struct IrTuple {
    /// Span of the tuple.
    pub span: Span,
    /// Arguments to construct the tuple.
    pub items: Box<[Ir]>,
}

/// Vector expression.
#[derive(Debug, Clone)]
pub struct IrVec {
    /// Span of the vector.
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
