//! Intermediate representation of Rune that can be evaluated in constant
//! contexts.
//!
//! This is part of the [Rune Language](https://rune-rs.github.io).

pub(crate) mod compiler;
mod eval;
mod interpreter;
pub(crate) mod scopes;
mod value;

use core::ops::{AddAssign, MulAssign, ShlAssign, ShrAssign, SubAssign};

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::{Box, Vec};
use crate::ast::{self, Span, Spanned};
use crate::compile::ir;
use crate::compile::{self, WithSpan};
use crate::hir;
use crate::indexing::index;
use crate::macros::MacroContext;
use crate::parse::NonZeroId;
use crate::query::Used;

pub(crate) use self::compiler::Ctxt;
pub(crate) use self::eval::{eval_ir, EvalOutcome};
pub(crate) use self::interpreter::{Budget, Interpreter};
pub(crate) use self::scopes::Scopes;
pub(crate) use self::value::Value;

impl ast::Expr {
    pub(crate) fn eval(&self, cx: &mut MacroContext<'_, '_, '_>) -> compile::Result<Value> {
        let mut expr = self.try_clone()?;
        index::expr(cx.idx, &mut expr)?;

        let ir = {
            // TODO: avoid this arena?
            let arena = hir::Arena::new();
            let mut hir_ctx = hir::lowering::Ctxt::with_const(
                &arena,
                cx.idx.q.borrow(),
                cx.item_meta.location.source_id,
            )?;
            let hir = hir::lowering::expr(&mut hir_ctx, &expr)?;

            let mut cx = Ctxt {
                source_id: cx.item_meta.location.source_id,
                q: cx.idx.q.borrow(),
            };

            compiler::expr(&hir, &mut cx)?
        };

        let mut ir_interpreter = Interpreter {
            budget: Budget::new(1_000_000),
            scopes: Scopes::new()?,
            module: cx.item_meta.module,
            item: cx.item_meta.item,
            q: cx.idx.q.borrow(),
        };

        ir_interpreter.eval_value(&ir, Used::Used)
    }
}

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
#[derive(Debug, TryClone, Spanned)]
pub(crate) struct Ir {
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
#[derive(Debug, TryClone, Spanned)]
pub(crate) struct IrTarget {
    /// Span of the target.
    #[rune(span)]
    pub(crate) span: Span,
    /// Kind of the target.
    pub(crate) kind: IrTargetKind,
}

/// The kind of the target.
#[derive(Debug, TryClone)]
pub(crate) enum IrTargetKind {
    /// A variable.
    Name(hir::OwnedName),
    /// A field target.
    Field(Box<IrTarget>, Box<str>),
    /// An index target.
    Index(Box<IrTarget>, usize),
}

decl_kind! {
    /// The kind of an intermediate operation.
    #[derive(Debug, TryClone)]
    pub(crate) enum IrKind {
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
        Name(hir::OwnedName),
        /// A local name. Could either be a local variable or a reference to
        /// something else, like another const declaration.
        Target(IrTarget),
        /// A constant value.
        Value(Value),
        /// A sequence of conditional branches.
        Branches(IrBranches),
        /// A loop.
        Loop(IrLoop),
        /// A break to the given target.
        Break(IrBreak),
        /// Constructing a vector.
        Vec(IrVec),
        /// Constructing a tuple.
        Tuple(Tuple),
        /// Constructing an object.
        Object(IrObject),
        /// A call.
        Call(IrCall),
    }
}

/// An interpeted function.
#[derive(Debug, TryClone, Spanned)]
pub(crate) struct IrFn {
    /// The span of the function.
    #[rune(span)]
    pub(crate) span: Span,
    /// The number of arguments the function takes and their names.
    pub(crate) args: Vec<hir::OwnedName>,
    /// The scope for the function.
    pub(crate) ir: Ir,
}

impl IrFn {
    pub(crate) fn compile_ast(
        hir: &hir::ItemFn<'_>,
        cx: &mut Ctxt<'_, '_>,
    ) -> compile::Result<Self> {
        let mut args = Vec::new();

        for arg in hir.args {
            if let hir::FnArg::Pat(hir::Pat {
                kind: hir::PatKind::Path(&hir::PatPathKind::Ident(name)),
                ..
            }) = arg
            {
                args.try_push(hir::Name::Str(name).into_owned()?)?;
                continue;
            }

            return Err(compile::Error::msg(arg, "Unsupported argument in const fn"));
        }

        let ir_scope = compiler::block(&hir.body, cx)?;

        Ok(ir::IrFn {
            span: hir.span(),
            args,
            ir: ir::Ir::new(hir.span(), ir_scope),
        })
    }
}

/// Definition of a new variable scope.
#[derive(Debug, TryClone, Spanned)]
pub(crate) struct IrScope {
    /// The span of the scope.
    #[rune(span)]
    pub(crate) span: Span,
    /// Instructions in the scope.
    pub(crate) instructions: Vec<Ir>,
    /// The implicit value of the scope.
    pub(crate) last: Option<Box<Ir>>,
}

/// A binary operation.
#[derive(Debug, TryClone, Spanned)]
pub(crate) struct IrBinary {
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
#[derive(Debug, TryClone, Spanned)]
pub(crate) struct IrDecl {
    /// The span of the declaration.
    #[rune(span)]
    pub(crate) span: Span,
    /// The name of the variable.
    pub(crate) name: hir::OwnedName,
    /// The value of the variable.
    pub(crate) value: Box<Ir>,
}

/// Set a target.
#[derive(Debug, TryClone, Spanned)]
pub(crate) struct IrSet {
    /// The span of the set operation.
    #[rune(span)]
    pub(crate) span: Span,
    /// The target to set.
    pub(crate) target: IrTarget,
    /// The value to set the target.
    pub(crate) value: Box<Ir>,
}

/// Assign a target.
#[derive(Debug, TryClone, Spanned)]
pub(crate) struct IrAssign {
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
#[derive(Debug, TryClone, Spanned)]
pub(crate) struct IrTemplate {
    /// The span of the template.
    #[rune(span)]
    pub(crate) span: Span,
    /// Template components.
    pub(crate) components: Vec<IrTemplateComponent>,
}

/// A string template.
#[derive(Debug, TryClone)]
pub(crate) enum IrTemplateComponent {
    /// An ir expression.
    Ir(Ir),
    /// A literal string.
    String(Box<str>),
}

/// Branch conditions in intermediate representation.
#[derive(Debug, TryClone, Spanned)]
pub(crate) struct IrBranches {
    /// Span associated with branches.
    #[rune(span)]
    pub(crate) span: Span,
    /// branches and their associated conditions.
    pub(crate) branches: Vec<(IrCondition, IrScope)>,
    /// The default fallback branch.
    pub(crate) default_branch: Option<IrScope>,
}

/// The condition for a branch.
#[derive(Debug, TryClone, Spanned)]
pub(crate) enum IrCondition {
    /// A simple conditional ir expression.
    Ir(Ir),
    /// A pattern match.
    Let(IrLet),
}

/// A pattern match.
#[derive(Debug, TryClone, Spanned)]
pub(crate) struct IrLet {
    /// The span of the let condition.
    #[rune(span)]
    pub(crate) span: Span,
    /// The pattern.
    pub(crate) pat: IrPat,
    /// The expression the pattern is evaluated on.
    pub(crate) ir: Ir,
}

/// A pattern.
#[derive(Debug, TryClone)]
pub(crate) enum IrPat {
    /// An ignore pattern `_`.
    Ignore,
    /// A named binding.
    Binding(hir::OwnedName),
}

impl IrPat {
    fn compile_ast(hir: &hir::Pat<'_>) -> compile::Result<Self> {
        match hir.kind {
            hir::PatKind::Ignore => return Ok(ir::IrPat::Ignore),
            hir::PatKind::Path(&hir::PatPathKind::Ident(name)) => {
                return Ok(ir::IrPat::Binding(hir::Name::Str(name).into_owned()?));
            }
            _ => (),
        }

        Err(compile::Error::msg(hir, "pattern not supported yet"))
    }

    fn matches<S>(
        &self,
        interp: &mut Interpreter<'_, '_>,
        value: Value,
        spanned: S,
    ) -> Result<bool, ir::EvalOutcome>
    where
        S: Spanned,
    {
        match self {
            IrPat::Ignore => Ok(true),
            IrPat::Binding(name) => {
                interp.scopes.decl(name, value).with_span(spanned)?;
                Ok(true)
            }
        }
    }
}

/// A loop with an optional condition.
#[derive(Debug, TryClone, Spanned)]
pub(crate) struct IrLoop {
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
#[derive(Debug, TryClone, Spanned)]
pub(crate) struct IrBreak {
    /// The span of the break.
    #[rune(span)]
    pub(crate) span: Span,
    /// The label of the break.
    pub(crate) label: Option<Box<str>>,
    /// The value of the break.
    pub(crate) expr: Option<Box<Ir>>,
}

impl IrBreak {
    fn compile_ast(
        span: Span,
        cx: &mut Ctxt<'_, '_>,
        hir: &hir::ExprBreak,
    ) -> compile::Result<Self> {
        let label = hir.label.map(TryInto::try_into).transpose()?;

        let expr = match hir.expr {
            Some(e) => Some(Box::try_new(compiler::expr(e, cx)?)?),
            None => None,
        };

        Ok(ir::IrBreak { span, label, expr })
    }

    /// Evaluate the break into an [ir::EvalOutcome].
    fn as_outcome(&self, interp: &mut Interpreter<'_, '_>, used: Used) -> ir::EvalOutcome {
        let span = self.span();

        if let Err(e) = interp.budget.take(span) {
            return e.into();
        }

        let expr = match &self.expr {
            Some(ir) => match ir::eval_ir(ir, interp, used) {
                Ok(value) => Some(value),
                Err(err) => return err,
            },
            None => None,
        };

        let label = match self.label.try_clone() {
            Ok(label) => label,
            Err(error) => return error.into(),
        };

        ir::EvalOutcome::Break(span, label, expr)
    }
}

/// Tuple expression.
#[derive(Debug, TryClone, Spanned)]
pub(crate) struct Tuple {
    /// Span of the tuple.
    #[rune(span)]
    pub(crate) span: Span,
    /// Arguments to construct the tuple.
    pub(crate) items: Box<[Ir]>,
}

/// Object expression.
#[derive(Debug, TryClone, Spanned)]
pub(crate) struct IrObject {
    /// Span of the object.
    #[rune(span)]
    pub(crate) span: Span,
    /// Field initializations.
    pub(crate) assignments: Box<[(Box<str>, Ir)]>,
}

/// Call expressions.
#[derive(Debug, TryClone, Spanned)]
pub(crate) struct IrCall {
    /// Span of the call.
    #[rune(span)]
    pub(crate) span: Span,
    /// The target of the call.
    pub(crate) id: NonZeroId,
    /// Arguments to the call.
    pub(crate) args: Vec<Ir>,
}

/// Vector expression.
#[derive(Debug, TryClone, Spanned)]
pub(crate) struct IrVec {
    /// Span of the vector.
    #[rune(span)]
    pub(crate) span: Span,
    /// Arguments to construct the vector.
    pub(crate) items: Box<[Ir]>,
}

/// A binary operation.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
pub(crate) enum IrBinaryOp {
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
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
pub(crate) enum IrAssignOp {
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
        target: &mut Value,
        operand: Value,
    ) -> compile::Result<()>
    where
        S: Copy + Spanned,
    {
        if let Value::Integer(target) = target {
            if let Value::Integer(operand) = operand {
                return self.assign_int(spanned, target, operand);
            }
        }

        Err(compile::Error::msg(spanned, "unsupported operands"))
    }

    /// Perform the given assign operation.
    fn assign_int<S>(self, spanned: S, target: &mut i64, operand: i64) -> compile::Result<()>
    where
        S: Copy + Spanned,
    {
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
                    .checked_div(operand)
                    .ok_or("division by zero")
                    .with_span(spanned)?;
            }
            IrAssignOp::Shl => {
                let operand = u32::try_from(operand)
                    .map_err(|_| "bad operand")
                    .with_span(spanned)?;

                target.shl_assign(operand);
            }
            IrAssignOp::Shr => {
                let operand = u32::try_from(operand)
                    .map_err(|_| "bad operand")
                    .with_span(spanned)?;

                target.shr_assign(operand);
            }
        }

        Ok(())
    }
}
