//! Helpers for building assembly.

use core::fmt;

use crate as rune;
use crate::alloc::fmt::TryWrite;
use crate::alloc::prelude::*;
use crate::alloc::{hash_map, HashMap};
use crate::alloc::{try_vec, String, Vec};
use crate::ast::{Span, Spanned};
use crate::compile::{self, Location};
use crate::runtime::{Inst, Label};
use crate::{Hash, SourceId};

#[derive(Debug, TryClone)]
pub(crate) enum AssemblyInst {
    Jump { label: Label },
    JumpIf { label: Label },
    JumpIfOrPop { label: Label },
    JumpIfNotOrPop { label: Label },
    JumpIfBranch { branch: i64, label: Label },
    PopAndJumpIfNot { count: usize, label: Label },
    IterNext { offset: usize, label: Label },
    Raw { raw: Inst },
}

/// Helper structure to build instructions and maintain certain invariants.
#[derive(Debug, TryClone, Default)]
pub(crate) struct Assembly {
    /// The location that caused the assembly.
    location: Location,
    /// Registered label by offset.
    pub(crate) labels: HashMap<usize, (usize, Vec<Label>)>,
    /// Instructions with spans.
    pub(crate) instructions: Vec<(AssemblyInst, Span)>,
    /// Comments associated with instructions.
    pub(crate) comments: HashMap<usize, String>,
    /// The number of labels.
    pub(crate) label_count: usize,
    /// The collection of functions required by this assembly.
    pub(crate) required_functions: HashMap<Hash, Vec<(Span, SourceId)>>,
}

impl Assembly {
    /// Construct a new assembly.
    pub(crate) fn new(location: Location, label_count: usize) -> Self {
        Self {
            location,
            labels: Default::default(),
            instructions: Default::default(),
            comments: Default::default(),
            label_count,
            required_functions: Default::default(),
        }
    }

    /// Construct and return a new label.
    pub(crate) fn new_label(&mut self, name: &'static str) -> Label {
        let label = Label::new(name, self.label_count);
        self.label_count += 1;
        label
    }

    /// Apply the label at the current instruction offset.
    pub(crate) fn label(&mut self, label: &Label) -> compile::Result<()> {
        let len = self.labels.len();

        match self.labels.entry(self.instructions.len()) {
            hash_map::Entry::Occupied(e) => {
                let &mut (len, ref mut labels) = e.into_mut();
                label.set_jump(len);
                labels.try_push(label.try_clone()?)?;
            }
            hash_map::Entry::Vacant(e) => {
                label.set_jump(len);
                e.try_insert((len, try_vec![label.try_clone()?]))?;
            }
        }

        Ok(())
    }

    /// Add a jump to the given label.
    pub(crate) fn jump(&mut self, label: &Label, span: &dyn Spanned) -> compile::Result<()> {
        self.inner_push(
            AssemblyInst::Jump {
                label: label.try_clone()?,
            },
            span,
        )?;

        Ok(())
    }

    /// Add a conditional jump to the given label.
    pub(crate) fn jump_if(&mut self, label: &Label, span: &dyn Spanned) -> compile::Result<()> {
        self.inner_push(
            AssemblyInst::JumpIf {
                label: label.try_clone()?,
            },
            span,
        )?;

        Ok(())
    }

    /// Add a conditional jump to the given label. Only pops the top of the
    /// stack if the jump is not executed.
    pub(crate) fn jump_if_or_pop(
        &mut self,
        label: &Label,
        span: &dyn Spanned,
    ) -> compile::Result<()> {
        self.inner_push(
            AssemblyInst::JumpIfOrPop {
                label: label.try_clone()?,
            },
            span,
        )?;

        Ok(())
    }

    /// Add a conditional jump to the given label. Only pops the top of the
    /// stack if the jump is not executed.
    pub(crate) fn jump_if_not_or_pop(
        &mut self,
        label: &Label,
        span: &dyn Spanned,
    ) -> compile::Result<()> {
        self.inner_push(
            AssemblyInst::JumpIfNotOrPop {
                label: label.try_clone()?,
            },
            span,
        )?;

        Ok(())
    }

    /// Add a conditional jump-if-branch instruction.
    pub(crate) fn jump_if_branch(
        &mut self,
        branch: i64,
        label: &Label,
        span: &dyn Spanned,
    ) -> compile::Result<()> {
        self.inner_push(
            AssemblyInst::JumpIfBranch {
                branch,
                label: label.try_clone()?,
            },
            span,
        )?;

        Ok(())
    }

    /// Add a pop-and-jump-if-not instruction to a label.
    pub(crate) fn pop_and_jump_if_not(
        &mut self,
        count: usize,
        label: &Label,
        span: &dyn Spanned,
    ) -> compile::Result<()> {
        self.inner_push(
            AssemblyInst::PopAndJumpIfNot {
                count,
                label: label.try_clone()?,
            },
            span,
        )?;

        Ok(())
    }

    /// Add an instruction that advanced an iterator.
    pub(crate) fn iter_next(
        &mut self,
        offset: usize,
        label: &Label,
        span: &dyn Spanned,
    ) -> compile::Result<()> {
        self.inner_push(
            AssemblyInst::IterNext {
                offset,
                label: label.try_clone()?,
            },
            span,
        )?;

        Ok(())
    }

    /// Push a raw instruction.
    pub(crate) fn push(&mut self, raw: Inst, span: &dyn Spanned) -> compile::Result<()> {
        if let Inst::Call { hash, .. } = raw {
            self.required_functions
                .entry(hash)
                .or_try_default()?
                .try_push((span.span(), self.location.source_id))?;
        }

        self.inner_push(AssemblyInst::Raw { raw }, span)?;
        Ok(())
    }

    /// Push a raw instruction.
    pub(crate) fn push_with_comment(
        &mut self,
        raw: Inst,
        span: &dyn Spanned,
        comment: &dyn fmt::Display,
    ) -> compile::Result<()> {
        let pos = self.instructions.len();

        let c = self.comments.entry(pos).or_try_default()?;

        if !c.is_empty() {
            c.try_push_str("; ")?;
        }

        write!(c, "{}", comment)?;
        self.push(raw, span)?;
        Ok(())
    }

    fn inner_push(&mut self, inst: AssemblyInst, span: &dyn Spanned) -> compile::Result<()> {
        self.instructions.try_push((inst, span.span()))?;
        Ok(())
    }
}
