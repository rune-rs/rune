use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::{self, Vec};
use crate::ast::Spanned;
use crate::compile::v1::Needs;
use crate::compile::{self, ErrorKind};
use crate::runtime::Label;

/// Loops we are inside.
#[derive(TryClone)]
pub(crate) struct Loop<'hir> {
    /// The optional label of the start of the loop.
    pub(crate) label: Option<&'hir str>,
    /// The start label of the loop, used for `continue`.
    pub(crate) continue_label: Label,
    /// The number of local variables inside the loop.
    pub(crate) continue_var_count: usize,
    /// The end label of the loop, used for `break`.
    pub(crate) break_label: Label,
    /// The number of local variables before the loop.
    pub(crate) break_var_count: usize,
    /// If the loop needs a value.
    pub(crate) needs: Needs,
    /// Locals to drop when breaking.
    pub(crate) drop: Option<usize>,
}

pub(crate) struct Loops<'hir> {
    loops: Vec<Loop<'hir>>,
}

impl<'hir> Loops<'hir> {
    /// Construct a new collection of loops.
    pub(crate) fn new() -> Self {
        Self { loops: Vec::new() }
    }

    /// Get the last loop context.
    pub(crate) fn last(&self) -> Option<&Loop<'hir>> {
        self.loops.last()
    }

    /// Push loop information.
    pub(crate) fn push(&mut self, l: Loop<'hir>) -> alloc::Result<()> {
        self.loops.try_push(l)?;
        Ok(())
    }

    pub(crate) fn pop(&mut self) {
        let empty = self.loops.pop().is_some();
        debug_assert!(empty);
    }

    /// Find the loop with the matching label.
    pub(crate) fn walk_until_label(
        &self,
        expected: &str,
        span: &dyn Spanned,
    ) -> compile::Result<(&Loop<'hir>, Vec<usize>)> {
        let mut to_drop = Vec::new();

        for l in self.loops.iter().rev() {
            to_drop.try_extend(l.drop)?;

            let Some(label) = l.label else {
                continue;
            };

            if expected == label {
                return Ok((l, to_drop));
            }
        }

        Err(compile::Error::new(
            span,
            ErrorKind::MissingLoopLabel {
                label: expected.try_into()?,
            },
        ))
    }

    /// Construct an iterator over all available scopes.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Loop<'hir>> {
        self.loops.iter()
    }
}
