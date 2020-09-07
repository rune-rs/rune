use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::error::CompileResult;
use crate::loops::Loop;
use crate::traits::{Compile, Resolve as _};
use runestick::Inst;

/// Compile a for loop.
impl Compile<(&ast::ExprFor, Needs)> for Compiler<'_> {
    fn compile(&mut self, (expr_for, needs): (&ast::ExprFor, Needs)) -> CompileResult<()> {
        let span = expr_for.span();
        log::trace!("ExprFor => {:?}", self.source.source(span));

        let start_label = self.asm.new_label("for_start");
        let end_label = self.asm.new_label("for_end");
        let break_label = self.asm.new_label("for_break");

        let total_var_count = self.scopes.last(span)?.total_var_count;

        let (iter_offset, loop_scope_expected) = {
            let mut loop_scope = self.scopes.child(span)?;
            self.compile((&*expr_for.iter, Needs::Value))?;

            let iter_offset = loop_scope.decl_anon(span);
            self.asm.push_with_comment(
                Inst::CallInstance {
                    hash: *runestick::INTO_ITER,
                    args: 0,
                },
                span,
                format!("into_iter (offset: {})", iter_offset),
            );

            let loop_scope_expected = self.scopes.push(loop_scope);
            (iter_offset, loop_scope_expected)
        };

        let _guard = self.loops.push(Loop {
            label: expr_for.label.map(|(label, _)| label),
            break_label,
            total_var_count,
            needs,
            drop: Some(iter_offset),
        });

        // Declare named loop variable.
        let binding_offset = {
            self.asm.push(Inst::Unit, expr_for.iter.span());
            let name = expr_for.var.resolve(&self.storage, &*self.source)?;
            self.scopes
                .last_mut(span)?
                .decl_var(name.as_ref(), expr_for.var.span())
        };

        // Declare storage for memoized `next` instance fn.
        let next_offset = if self.options.memoize_instance_fn {
            let span = expr_for.iter.span();

            let offset = self.scopes.decl_anon(span)?;

            // Declare the named loop variable and put it in the scope.
            self.asm.push_with_comment(
                Inst::Copy {
                    offset: iter_offset,
                },
                span,
                "copy iterator (memoize)",
            );

            self.asm.push_with_comment(
                Inst::LoadInstanceFn {
                    hash: *runestick::NEXT,
                },
                span,
                "load instance fn (memoize)",
            );

            Some(offset)
        } else {
            None
        };

        self.asm.label(start_label)?;

        // Use the memoized loop variable.
        if let Some(next_offset) = next_offset {
            self.asm.push_with_comment(
                Inst::Copy {
                    offset: iter_offset,
                },
                expr_for.iter.span(),
                "copy iterator",
            );

            self.asm.push_with_comment(
                Inst::Copy {
                    offset: next_offset,
                },
                expr_for.iter.span(),
                "copy next",
            );

            self.asm.push(Inst::CallFn { args: 1 }, span);

            self.asm.push(
                Inst::Replace {
                    offset: binding_offset,
                },
                expr_for.var.span(),
            );
        } else {
            // call the `next` function to get the next level of iteration, bind the
            // result to the loop variable in the loop.
            self.asm.push(
                Inst::Copy {
                    offset: iter_offset,
                },
                expr_for.iter.span(),
            );

            self.asm.push_with_comment(
                Inst::CallInstance {
                    hash: *runestick::NEXT,
                    args: 0,
                },
                span,
                "next",
            );
            self.asm.push(
                Inst::Replace {
                    offset: binding_offset,
                },
                expr_for.var.span(),
            );
        }

        // test loop condition and unwrap the option.
        // TODO: introduce a dedicated instruction for this :|.
        {
            self.asm.push(
                Inst::Copy {
                    offset: binding_offset,
                },
                expr_for.var.span(),
            );
            self.asm.push(Inst::IsValue, expr_for.span());
            self.asm.jump_if_not(end_label, expr_for.span());
            self.asm.push(
                Inst::Copy {
                    offset: binding_offset,
                },
                expr_for.var.span(),
            );
            // unwrap the optional value.
            self.asm.push(Inst::Unwrap, expr_for.span());
            self.asm.push(
                Inst::Replace {
                    offset: binding_offset,
                },
                expr_for.var.span(),
            );
        }

        self.compile((&*expr_for.body, Needs::None))?;
        self.asm.jump(start_label, span);
        self.asm.label(end_label)?;

        // Drop the iterator.
        self.asm.push(
            Inst::Drop {
                offset: iter_offset,
            },
            span,
        );

        self.clean_last_scope(span, loop_scope_expected, Needs::None)?;

        // NB: If a value is needed from a for loop, encode it as a unit.
        if needs.value() {
            self.asm.push(Inst::Unit, span);
        }

        // NB: breaks produce their own value.
        self.asm.label(break_label)?;
        Ok(())
    }
}
