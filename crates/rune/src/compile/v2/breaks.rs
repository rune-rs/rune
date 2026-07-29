use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::{self, Vec};
use crate::ast::Spanned;
use crate::compile::{self, ErrorKind, WithSpan};
use crate::runtime::{Address, Label, Output};

/// Loops we are inside.
#[derive(TryClone)]
pub(crate) struct Break<'hir> {
    /// The optional label of the start of the break.
    pub(crate) label: Option<&'hir str>,
    /// If the break supports breaking with a value, this would be where to
    /// store it.
    pub(crate) output: Option<Output>,
    /// If the break supports continuing, this is the label to use.
    pub(crate) continue_label: Option<Label>,
    /// The end label of the break, used for `break`.
    pub(crate) break_label: Label,
    /// Locals to drop when breaking.
    pub(crate) drop: Option<Address>,
}

pub(crate) struct Breaks<'hir> {
    loops: Vec<Break<'hir>>,
}

impl<'hir> Breaks<'hir> {
    /// Construct a new collection of loops.
    pub(crate) fn new() -> Self {
        Self { loops: Vec::new() }
    }

    /// Get the last loop context.
    pub(crate) fn last(&self) -> Option<&Break<'hir>> {
        self.loops.last()
    }

    /// Push loop information.
    pub(crate) fn push(&mut self, l: Break<'hir>) -> alloc::Result<()> {
        self.loops.try_push(l)?;
        Ok(())
    }

    pub(crate) fn pop(&mut self) {
        let empty = self.loops.pop().is_some();
        debug_assert!(empty);
    }

    /// Find the loop with the matching label and collect addresses to drop.
    pub(crate) fn walk_until_label(
        &self,
        span: &dyn Spanned,
        expected: &str,
        drop: &mut Vec<Address>,
    ) -> compile::Result<&Break<'hir>> {
        drop.clear();
        self.find_label_inner(span, expected, &mut |l| drop.try_extend(l.drop))
    }

    /// Find the loop with the matching label.
    pub(crate) fn find_label(
        &self,
        span: &dyn Spanned,
        expected: &str,
    ) -> compile::Result<&Break<'hir>> {
        self.find_label_inner(span, expected, &mut |_| Ok(()))
    }

    /// Find the loop with the matching label.
    fn find_label_inner(
        &self,
        span: &dyn Spanned,
        expected: &str,
        visitor: &mut dyn FnMut(&Break<'hir>) -> alloc::Result<()>,
    ) -> compile::Result<&Break<'hir>> {
        for l in self.loops.iter().rev() {
            visitor(l).with_span(span)?;

            let Some(label) = l.label else {
                continue;
            };

            if expected == label {
                return Ok(l);
            }
        }

        Err(compile::Error::new(
            span,
            ErrorKind::MissingLabel {
                label: expected.try_into()?,
            },
        ))
    }
}
