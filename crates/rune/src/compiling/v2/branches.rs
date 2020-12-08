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
                cond: Some((cond, cond_block)),
            });
        }

        if let Some(code) = self.fallback {
            let code_block = first
                .take()
                .unwrap_or_else(|| c.program.named("branches_fallback_code"));

            blocks.push(Entry {
                code,
                code_block,
                cond: None,
            });
        }

        let output = c.program.var();
        let output_block = c.program.named("branches_output");

        // queue of blocks to seal.
        let mut to_seal = Vec::new();

        for window in blocks.windows(2) {
            if let [from, to] = window {
                if let Some((cond, cond_block)) = &from.cond {
                    let (cond_block, cond) = cond.assemble(c, cond_block.clone())?;

                    cond_block
                        .jump_if(cond, &from.code_block, to.cond_block())
                        .with_span(span)?;

                    to_seal.push(cond_block);
                }

                let (value_block, value) = from.code.assemble(c, from.code_block.clone())?;
                value_block.assign(output, value).with_span(span)?;
                value_block.jump(&output_block).with_span(span)?;
                to_seal.push(value_block);
            }
        }

        if let Some(last) = blocks.last() {
            if let Some((cond, cond_block)) = &last.cond {
                let (cond_block, cond) = cond.assemble(c, cond_block.clone())?;

                cond_block
                    .jump_if(cond, &last.code_block, &output_block)
                    .with_span(span)?;

                to_seal.push(cond_block);
            }

            let (value_block, value) = last.code.assemble(c, last.code_block.clone())?;
            value_block.assign(output, value).with_span(span)?;
            value_block.jump(&output_block).with_span(span)?;
            to_seal.push(value_block);
        }

        for block in to_seal {
            block.seal().with_span(span)?;
        }

        Ok((output_block, output))
    }
}

struct Entry<'a> {
    code: &'a ast::Block,
    code_block: Block,
    cond: Option<(&'a ast::Condition, Block)>,
}

impl<'a> Entry<'a> {
    fn cond_block(&self) -> &Block {
        match &self.cond {
            Some((_, cond)) => cond,
            None => &self.code_block,
        }
    }
}
