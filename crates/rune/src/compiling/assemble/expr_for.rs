use crate::compiling::assemble::prelude::*;

/// Compile a for loop.
impl Assemble for ast::ExprFor {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprFor => {:?}", c.source.source(span));

        let start_label = c.asm.new_label("for_start");
        let end_label = c.asm.new_label("for_end");
        let break_label = c.asm.new_label("for_break");

        let total_var_count = c.scopes.total_var_count(span)?;

        let (iter_offset, loop_scope_expected) = {
            let loop_scope_expected = c.scopes.push_child(span)?;
            self.iter.assemble(c, Needs::Value)?;

            let iter_offset = c.scopes.decl_anon(span)?;
            c.asm.push_with_comment(
                Inst::CallInstance {
                    hash: *runestick::Protocol::INTO_ITER,
                    args: 0,
                },
                span,
                format!("into_iter (offset: {})", iter_offset),
            );

            (iter_offset, loop_scope_expected)
        };

        let _guard = c.loops.push(Loop {
            label: self.label.map(|(label, _)| label),
            break_label,
            total_var_count,
            needs,
            drop: Some(iter_offset),
        });

        // Declare named loop variable.
        let binding_offset = {
            c.asm.push(Inst::unit(), self.iter.span());
            let name = self.var.resolve(&c.storage, &*c.source)?;
            c.scopes.decl_var(name.as_ref(), self.var.span())?
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
                    hash: *runestick::Protocol::NEXT,
                },
                span,
                "load instance fn (memoize)",
            );

            Some(offset)
        } else {
            None
        };

        c.asm.label(start_label)?;

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
                self.var.span(),
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
                    hash: *runestick::Protocol::NEXT,
                    args: 0,
                },
                span,
                "next",
            );
            c.asm.push(
                Inst::Replace {
                    offset: binding_offset,
                },
                self.var.span(),
            );
        }

        // test loop condition and unwrap the option.
        // TODO: introduce a dedicated instruction for this :|.
        {
            c.asm.push(
                Inst::Copy {
                    offset: binding_offset,
                },
                self.var.span(),
            );
            c.asm.push(Inst::IsValue, self.span());
            c.asm.jump_if_not(end_label, self.span());
            c.asm.push(
                Inst::Copy {
                    offset: binding_offset,
                },
                self.var.span(),
            );
            // unwrap the optional value.
            c.asm.push(Inst::Unwrap, self.span());
            c.asm.push(
                Inst::Replace {
                    offset: binding_offset,
                },
                self.var.span(),
            );
        }

        self.body.assemble(c, Needs::None)?;
        c.asm.jump(start_label, span);
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
        Ok(())
    }
}
