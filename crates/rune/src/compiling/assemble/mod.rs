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
mod expr_field_access;
mod expr_for;
mod expr_if;
mod expr_index;
mod expr_let;
mod expr_loop;
mod expr_match;
mod expr_object;
mod expr_path;
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

use runestick::{CompileMetaCapture, Span};

/// Compiler trait implemented for things that can be compiled.
///
/// This is the new compiler trait to implement.
pub(crate) trait Assemble {
    /// Walk the current type with the given item.
    fn assemble(
        &self,
        c: &mut crate::compiling::Compiler<'_>,
        needs: crate::compiling::Needs,
    ) -> crate::compiling::CompileResult<()>;
}

/// Assemble a constant.
pub(crate) trait AssembleConst {
    fn assemble_const(
        &self,
        c: &mut crate::compiling::Compiler<'_>,
        needs: crate::compiling::Needs,
        span: Span,
    ) -> crate::compiling::CompileResult<()>;
}

/// Assemble a function.
pub(crate) trait AssembleFn {
    /// Walk the current type with the given item.
    fn assemble_fn(
        &self,
        c: &mut crate::compiling::Compiler<'_>,
        instance_fn: bool,
    ) -> crate::compiling::CompileResult<()>;
}

/// Assemble a closure with captures.
pub(crate) trait AssembleClosure {
    fn assemble_closure(
        &self,
        c: &mut crate::compiling::Compiler<'_>,
        captures: &[CompileMetaCapture],
    ) -> crate::compiling::CompileResult<()>;
}
