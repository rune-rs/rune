use crate::compiling::v1::assemble::prelude::*;

/// Compile a for loop.
impl Assemble for ast::ExprFor {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Asm> {
        let span = self.span();
        log::trace!("ExprFor => {:?}", c.source.source(span));

        let continue_label = c.asm.new_label("for_continue");
        let end_label = c.asm.new_label("for_end");
        let break_label = c.asm.new_label("for_break");

        let break_var_count = c.scopes.total_var_count(span)?;

        let (iter_offset, loop_scope_expected) = {
            let loop_scope_expected = c.scopes.push_child(span)?;
            self.iter.assemble(c, Needs::Value)?.apply(c)?;

            let iter_offset = c.scopes.decl_anon(span)?;
            c.asm.push_with_comment(
                Inst::CallInstance {
                    hash: *Protocol::INTO_ITER,
                    args: 0,
                },
                span,
                format!("into_iter (offset: {})", iter_offset),
            );

            (iter_offset, loop_scope_expected)
        };

        let binding_span = self.binding.span();

        // Declare named loop variable.
        let binding_offset = {
            c.asm.push(Inst::unit(), self.iter.span());
            c.scopes.decl_anon(binding_span)?
        };

        // Declare storage for memoized `next` instance fn.
        let next_offset = if c.options.memoize_instance_fn {
            let span = self.iter.span();

            let offset = c.scopes.decl_anon(span)?;

            // Declare the named loop variable and put it in the scope.
            c.asm.push_with_comment(
                Inst::Copy {
                    offset: iter_offset,
                },
                span,
                "copy iterator (memoize)",
            );

            c.asm.push_with_comment(
                Inst::LoadInstanceFn {
                    hash: *Protocol::NEXT,
                },
                span,
                "load instance fn (memoize)",
            );

            Some(offset)
        } else {
            None
        };

        let continue_var_count = c.scopes.total_var_count(span)?;
        c.asm.label(continue_label)?;

        let _guard = c.loops.push(Loop {
            label: self.label.map(|(label, _)| label),
            continue_label,
            continue_var_count,
            break_label,
            break_var_count,
            needs,
            drop: Some(iter_offset),
        });

        // Use the memoized loop variable.
        if let Some(next_offset) = next_offset {
            c.asm.push_with_comment(
                Inst::Copy {
                    offset: iter_offset,
                },
                self.iter.span(),
                "copy iterator",
            );

            c.asm.push_with_comment(
                Inst::Copy {
                    offset: next_offset,
                },
                self.iter.span(),
                "copy next",
            );

            c.asm.push(Inst::CallFn { args: 1 }, span);

            c.asm.push(
                Inst::Replace {
                    offset: binding_offset,
                },
                binding_span,
            );
        } else {
            // call the `next` function to get the next level of iteration, bind the
            // result to the loop variable in the loop.
            c.asm.push(
                Inst::Copy {
                    offset: iter_offset,
                },
                self.iter.span(),
            );

            c.asm.push_with_comment(
                Inst::CallInstance {
                    hash: *Protocol::NEXT,
                    args: 0,
                },
                span,
                "next",
            );
            c.asm.push(
                Inst::Replace {
                    offset: binding_offset,
                },
                binding_span,
            );
        }

        // Test loop condition and unwrap the option, or jump to `end_label` if the current value is `None`.
        c.asm.iter_next(binding_offset, end_label, binding_span);

        let body_span = self.body.span();
        let guard = c.scopes.push_child(body_span)?;

        c.compile_pat_offset(&self.binding, binding_offset)?;

        self.body.assemble(c, Needs::None)?.apply(c)?;
        c.clean_last_scope(span, guard, Needs::None)?;

        c.asm.jump(continue_label, span);
        c.asm.label(end_label)?;

        // Drop the iterator.
        c.asm.push(
            Inst::Drop {
                offset: iter_offset,
            },
            span,
        );

        c.clean_last_scope(span, loop_scope_expected, Needs::None)?;

        // NB: If a value is needed from a for loop, encode it as a unit.
        if needs.value() {
            c.asm.push(Inst::unit(), span);
        }

        // NB: breaks produce their own value.
        c.asm.label(break_label)?;
        Ok(Asm::top(span))
    }
}
