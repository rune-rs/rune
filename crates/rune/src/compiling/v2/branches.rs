use crate::ast;
use crate::compiling::v2::{Assemble as _, Compiler};
use crate::compiling::CompileError;
use crate::shared::ResultExt as _;
use rune_ssa::{Block, Var};
use runestick::Span;

pub(crate) struct Branches<'a> {
    conditional: Vec<(&'a ast::Block, &'a ast::Condition)>,
    fallback: Option<&'a ast::Block>,
}

impl<'a> Branches<'a> {
    /// Construct a new branch builder.
    pub(crate) fn new() -> Self {
        Self {
            conditional: Vec::new(),
            fallback: None,
        }
    }

    /// Set up a conditional block.
    pub(crate) fn conditional(&mut self, block: &'a ast::Block, c: &'a ast::Condition) {
        self.conditional.push((block, c));
    }

    /// Set up a fallback block.
    pub(crate) fn fallback(&mut self, block: &'a ast::Block) {
        self.fallback = Some(block);
    }

    pub(crate) fn assemble(
        self,
        span: Span,
        c: &mut Compiler<'_>,
        block: Block,
    ) -> Result<(Block, Var), CompileError> {
        let mut blocks = Vec::new();

        let mut first = Some(block);

        for (n, (code, cond)) in self.conditional.into_iter().enumerate() {
            let code_block = c.program.named(&format!("branches_code_{}", n));
            let cond_block = first
                .take()
                .unwrap_or_else(|| c.program.named(&format!("branches_cond_{}", n)));

            blocks.push(Entry {
                code,
                code_block,
                cond,
                cond_block,
            });
        }

        let fallback_block = first
            .take()
            .unwrap_or_else(|| c.program.named("branches_fallback_code"));

        let output = c.program.var();
        let output_block = c.program.named("branches_output");

        let fallback_block = if let Some(code) = self.fallback {
            let (fallback_block, var) = code.assemble(c, fallback_block)?;
            fallback_block.assign(output, var).with_span(span)?;
            fallback_block.jump(&output_block).with_span(span)?;
            fallback_block
        } else {
            let unit = fallback_block.unit().with_span(span)?;
            fallback_block.assign(output, unit).with_span(span)?;
            fallback_block.jump(&output_block).with_span(span)?;
            fallback_block
        };

        let mut it = blocks.into_iter().peekable();

        if let Some(entry) = it.next() {
            let Entry {
                code_block,
                code,
                cond_block,
                cond,
            } = entry;

            let else_block = if let Some(to) = it.peek() {
                &to.cond_block
            } else {
                &fallback_block
            };

            let (cond_block, cond) = cond.assemble(c, cond_block)?;
            cond_block
                .jump_if(cond, &code_block, else_block)
                .with_span(span)?;

            let (code_block, var) = code.assemble(c, code_block)?;
            println!("{} = {} ({:?})", output, var, code_block.name());
            code_block.assign(output, var).with_span(span)?;
            code_block.jump(&output_block).with_span(span)?;
        }

        Ok((output_block, output))
    }
}

struct Entry<'a> {
    code: &'a ast::Block,
    code_block: Block,
    cond: &'a ast::Condition,
    cond_block: Block,
}
