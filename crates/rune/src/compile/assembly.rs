//! Helpers for building assembly.

use core::fmt;

use crate::no_std::collections::{hash_map, HashMap};
use crate::no_std::prelude::*;

use crate::ast::{Span, Spanned};
use crate::compile::{self, Location};
use crate::runtime::{Inst, Label};
use crate::{Hash, SourceId};

#[derive(Debug, Clone)]
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
#[derive(Debug, Clone, Default)]
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
                labels.push(label.clone());
            }
            hash_map::Entry::Vacant(e) => {
                label.set_jump(len);
                e.insert((len, vec![label.clone()]));
            }
        }

        Ok(())
    }

    /// Add a jump to the given label.
    pub(crate) fn jump(&mut self, label: &Label, span: &dyn Spanned) {
        self.inner_push(
            AssemblyInst::Jump {
                label: label.clone(),
            },
            span,
        );
    }

    /// Add a conditional jump to the given label.
    pub(crate) fn jump_if(&mut self, label: &Label, span: &dyn Spanned) {
        self.inner_push(
            AssemblyInst::JumpIf {
                label: label.clone(),
            },
            span,
        );
    }

    /// Add a conditional jump to the given label. Only pops the top of the
    /// stack if the jump is not executed.
    pub(crate) fn jump_if_or_pop(&mut self, label: &Label, span: &dyn Spanned) {
        self.inner_push(
            AssemblyInst::JumpIfOrPop {
                label: label.clone(),
            },
            span,
        );
    }

    /// Add a conditional jump to the given label. Only pops the top of the
    /// stack if the jump is not executed.
    pub(crate) fn jump_if_not_or_pop(&mut self, label: &Label, span: &dyn Spanned) {
        self.inner_push(
            AssemblyInst::JumpIfNotOrPop {
                label: label.clone(),
            },
            span,
        );
    }

    /// Add a conditional jump-if-branch instruction.
    pub(crate) fn jump_if_branch(&mut self, branch: i64, label: &Label, span: &dyn Spanned) {
        self.inner_push(
            AssemblyInst::JumpIfBranch {
                branch,
                label: label.clone(),
            },
            span,
        );
    }

    /// Add a pop-and-jump-if-not instruction to a label.
    pub(crate) fn pop_and_jump_if_not(&mut self, count: usize, label: &Label, span: &dyn Spanned) {
        self.inner_push(
            AssemblyInst::PopAndJumpIfNot {
                count,
                label: label.clone(),
            },
            span,
        );
    }

    /// Add an instruction that advanced an iterator.
    pub(crate) fn iter_next(&mut self, offset: usize, label: &Label, span: &dyn Spanned) {
        self.inner_push(
            AssemblyInst::IterNext {
                offset,
                label: label.clone(),
            },
            span,
        );
    }

    /// Push a raw instruction.
    pub(crate) fn push(&mut self, raw: Inst, span: &dyn Spanned) {
        if let Inst::Call { hash, .. } = raw {
            self.required_functions
                .entry(hash)
                .or_default()
                .push((span.span(), self.location.source_id));
        }

        self.inner_push(AssemblyInst::Raw { raw }, span);
    }

    /// Push a raw instruction.
    pub(crate) fn push_with_comment(
        &mut self,
        raw: Inst,
        span: &dyn Spanned,
        comment: &dyn fmt::Display,
    ) -> compile::Result<()> {
        use core::fmt::Write;

        let pos = self.instructions.len();

        let c = self.comments.entry(pos).or_default();

        if !c.is_empty() {
            c.push_str("; ");
        }

        if let Err(fmt::Error) = write!(c, "{}", comment) {
            return Err(compile::Error::msg(span, "Failed to write comment"));
        }

        self.push(raw, span);
        Ok(())
    }

    fn inner_push(&mut self, inst: AssemblyInst, span: &dyn Spanned) {
        self.instructions.push((inst, span.span()));
    }
}
