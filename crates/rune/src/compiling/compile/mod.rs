mod block;
mod const_value;
mod expr;
mod expr_async;
mod expr_await;
mod expr_binary;
mod expr_block;
mod expr_break;
mod expr_call;
mod expr_closure;
mod expr_field_access;
mod expr_for;
mod expr_if;
mod expr_index_get;
mod expr_index_set;
mod expr_let;
mod expr_loop;
mod expr_match;
mod expr_path;
mod expr_return;
mod expr_select;
mod expr_self;
mod expr_try;
mod expr_unary;
mod expr_while;
mod expr_yield;
mod item_fn;
mod lit_bool;
mod lit_byte;
mod lit_byte_str;
mod lit_char;
mod lit_number;
mod lit_object;
mod lit_str;
mod lit_template;
mod lit_tuple;
mod lit_unit;
mod lit_vec;
mod prelude;

/// Compiler trait implemented for things that can be compiled.
pub(crate) trait Compile<T> {
    /// Walk the current type with the given item.
    fn compile(&mut self, item: T) -> crate::compiling::CompileResult<()>;
}
