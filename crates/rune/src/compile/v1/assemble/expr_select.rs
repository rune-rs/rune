use crate::compile::v1::assemble::prelude::*;

/// Compile a select expression.
impl Assemble for ast::ExprSelect {
    fn assemble(&self, c: &mut Compiler<'_, '_>, needs: Needs) -> CompileResult<Asm> {
        let span = self.span();
        log::trace!("ExprSelect => {:?}", c.q.sources.source(c.source_id, span));
        let len = self.branches.len();
        c.contexts.push(span);

        let mut default_branch = None;
        let mut branches = Vec::new();

        let end_label = c.asm.new_label("select_end");

        for (branch, _) in &self.branches {
            match branch {
                ast::ExprSelectBranch::Pat(pat) => {
                    let label = c.asm.new_label("select_branch");
                    branches.push((label, pat));
                }
                ast::ExprSelectBranch::Default(def) => {
                    if default_branch.is_some() {
                        return Err(CompileError::new(
                            span,
                            CompileErrorKind::SelectMultipleDefaults,
                        ));
                    }

                    let label = c.asm.new_label("select_default");
                    default_branch = Some((def, label));
                }
            }
        }

        for (_, branch) in &branches {
            branch.expr.assemble(c, Needs::Value)?.apply(c)?;
        }

        c.asm.push(Inst::Select { len }, span);

        for (branch, (label, _)) in branches.iter().enumerate() {
            c.asm.jump_if_branch(branch as i64, *label, span);
        }

        if let Some((_, label)) = &default_branch {
            c.asm.push(Inst::Pop, span);
            c.asm.jump(*label, span);
        }

        if !needs.value() {
            c.asm.push(Inst::Pop, span);
        }

        c.asm.jump(end_label, span);

        for (label, branch) in branches {
            let span = branch.span();
            c.asm.label(label)?;

            let expected = c.scopes.push_child(span)?;

            // NB: loop is actually useful.
            #[allow(clippy::never_loop)]
            loop {
                match &branch.pat {
                    ast::Pat::PatPath(path) => {
                        let named = c.convert_path_to_named(&path.path)?;

                        if let Some(local) = named.as_local() {
                            c.scopes.decl_var(local, path.span())?;
                            break;
                        }
                    }
                    ast::Pat::PatIgnore(..) => {
                        c.asm.push(Inst::Pop, span);
                        break;
                    }
                    _ => (),
                }

                return Err(CompileError::new(
                    branch,
                    CompileErrorKind::UnsupportedSelectPattern,
                ));
            }

            // Set up a new scope with the binding.
            branch.body.assemble(c, needs)?.apply(c)?;
            c.clean_last_scope(span, expected, needs)?;
            c.asm.jump(end_label, span);
        }

        if let Some((branch, label)) = default_branch {
            c.asm.label(label)?;
            branch.body.assemble(c, needs)?.apply(c)?;
        }

        c.asm.label(end_label)?;

        c.contexts
            .pop()
            .ok_or_else(|| CompileError::msg(&span, "missing parent context"))?;

        Ok(Asm::top(span))
    }
}
