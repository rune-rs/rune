mod block;
mod builtin_format;
mod builtin_template;
mod const_value;
mod expr;
mod expr_assign;
mod expr_await;
mod expr_binary;
mod expr_block;
mod expr_break;
mod expr_call;
mod expr_closure;
mod expr_continue;
mod expr_field_access;
mod expr_for;
mod expr_if;
mod expr_index;
mod expr_let;
mod expr_loop;
mod expr_match;
mod expr_object;
mod expr_path;
mod expr_range;
mod expr_return;
mod expr_select;
mod expr_try;
mod expr_tuple;
mod expr_unary;
mod expr_vec;
mod expr_while;
mod expr_yield;
mod item_fn;
mod lit;
mod lit_bool;
mod lit_byte;
mod lit_byte_str;
mod lit_char;
mod lit_number;
mod lit_str;
mod local;
mod prelude;

use crate::ast::Span;
use crate::compile::v1::{Compiler, Needs, Var};
use crate::compile::{CaptureMeta, CompileResult};
use crate::runtime::InstAddress;

#[derive(Debug)]
#[must_use = "must be consumed to make sure the value is realized"]
pub(crate) struct Asm {
    span: Span,
    kind: AsmKind,
}

impl Asm {
    /// Construct an assembly result that leaves the value on the top of the
    /// stack.
    pub(crate) fn top(span: Span) -> Self {
        Self {
            span,
            kind: AsmKind::Top,
        }
    }

    pub(crate) fn var(span: Span, var: Var, local: Box<str>) -> Self {
        Self {
            span,
            kind: AsmKind::Var(var, local),
        }
    }
}

#[derive(Debug)]
pub(crate) enum AsmKind {
    // Result is pushed onto the top of the stack.
    Top,
    // Result belongs to the the given stack offset.
    Var(Var, Box<str>),
}

impl Asm {
    /// Assemble into an instruction.
    pub(crate) fn apply(self, c: &mut Compiler) -> CompileResult<()> {
        match self.kind {
            AsmKind::Top => (),
            AsmKind::Var(var, local) => {
                var.copy(&mut c.asm, self.span, format!("var `{}`", local));
            }
        }

        Ok(())
    }

    /// Assemble into an instruction declaring an anonymous variable if appropriate.
    pub(crate) fn apply_targeted(self, c: &mut Compiler) -> CompileResult<InstAddress> {
        let address = match self.kind {
            AsmKind::Top => {
                c.scopes.decl_anon(self.span)?;
                InstAddress::Top
            }
            AsmKind::Var(var, ..) => InstAddress::Offset(var.offset),
        };

        Ok(address)
    }
}

/// Compiler trait implemented for things that can be compiled.
///
/// This is the new compiler trait to implement.
pub(crate) trait Assemble {
    /// Walk the current type with the given item.
    fn assemble(&self, c: &mut Compiler<'_, '_>, needs: Needs) -> CompileResult<Asm>;
}

/// Assemble a constant.
pub(crate) trait AssembleConst {
    fn assemble_const(
        &self,
        c: &mut Compiler<'_, '_>,
        needs: Needs,
        span: Span,
    ) -> CompileResult<()>;
}

/// Assemble a function.
pub(crate) trait AssembleFn {
    /// Walk the current type with the given item.
    fn assemble_fn(&self, c: &mut Compiler<'_, '_>, instance_fn: bool) -> CompileResult<()>;
}

/// Assemble a closure with captures.
pub(crate) trait AssembleClosure {
    fn assemble_closure(
        &self,
        c: &mut Compiler<'_, '_>,
        captures: &[CaptureMeta],
    ) -> CompileResult<()>;
}
