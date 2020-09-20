//! Helpers for building assembly.

use crate::collections::HashMap;
use crate::unit_builder::UnitBuilderError;
use runestick::{Hash, Inst, Label, Span};

#[derive(Debug, Clone)]
pub enum AssemblyInst {
    Jump { label: Label },
    JumpIf { label: Label },
    JumpIfNot { label: Label },
    JumpIfOrPop { label: Label },
    JumpIfNotOrPop { label: Label },
    JumpIfBranch { branch: i64, label: Label },
    PopAndJumpIfNot { count: usize, label: Label },
    Raw { raw: Inst },
}

/// Helper structure to build instructions and maintain certain invariants.
#[derive(Debug, Clone, Default)]
pub struct Assembly {
    /// The source id of the assembly.
    pub(crate) source_id: usize,
    /// Label to offset.
    pub(crate) labels: HashMap<Label, usize>,
    /// Registered label by offset.
    pub(crate) labels_rev: HashMap<usize, Label>,
    /// Instructions with spans.
    pub(crate) instructions: Vec<(AssemblyInst, Span)>,
    /// Comments associated with instructions.
    pub(crate) comments: HashMap<usize, Vec<String>>,
    /// The number of labels.
    pub(crate) label_count: usize,
    /// The collection of functions required by this assembly.
    pub(crate) required_functions: HashMap<Hash, Vec<(Span, usize)>>,
}

impl Assembly {
    /// Construct a new assembly.
    pub(crate) fn new(source_id: usize, label_count: usize) -> Self {
        Self {
            source_id,
            labels: Default::default(),
            labels_rev: Default::default(),
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
    pub(crate) fn label(&mut self, label: Label) -> Result<Label, UnitBuilderError> {
        let offset = self.instructions.len();

        if self.labels.insert(label, offset).is_some() {
            return Err(UnitBuilderError::DuplicateLabel { label });
        }

        self.labels_rev.insert(offset, label);
        Ok(label)
    }

    /// Add a jump to the given label.
    pub(crate) fn jump(&mut self, label: Label, span: Span) {
        self.instructions.push((AssemblyInst::Jump { label }, span));
    }

    /// Add a conditional jump to the given label.
    pub(crate) fn jump_if(&mut self, label: Label, span: Span) {
        self.instructions
            .push((AssemblyInst::JumpIf { label }, span));
    }

    /// Add a conditional jump to the given label.
    pub(crate) fn jump_if_not(&mut self, label: Label, span: Span) {
        self.instructions
            .push((AssemblyInst::JumpIfNot { label }, span));
    }

    /// Add a conditional jump to the given label. Only pops the top of the
    /// stack if the jump is not executed.
    pub(crate) fn jump_if_or_pop(&mut self, label: Label, span: Span) {
        self.instructions
            .push((AssemblyInst::JumpIfOrPop { label }, span));
    }

    /// Add a conditional jump to the given label. Only pops the top of the
    /// stack if the jump is not executed.
    pub(crate) fn jump_if_not_or_pop(&mut self, label: Label, span: Span) {
        self.instructions
            .push((AssemblyInst::JumpIfNotOrPop { label }, span));
    }

    /// Add a conditional jump-if-branch instruction.
    pub(crate) fn jump_if_branch(&mut self, branch: i64, label: Label, span: Span) {
        self.instructions
            .push((AssemblyInst::JumpIfBranch { branch, label }, span));
    }

    /// Add a pop-and-jump-if-not instruction to a label.
    pub(crate) fn pop_and_jump_if_not(&mut self, count: usize, label: Label, span: Span) {
        self.instructions
            .push((AssemblyInst::PopAndJumpIfNot { count, label }, span));
    }

    /// Push a raw instruction.
    pub(crate) fn push(&mut self, raw: Inst, span: Span) {
        if let Inst::Call { hash, .. } = raw {
            self.required_functions
                .entry(hash)
                .or_default()
                .push((span, self.source_id));
        }

        self.instructions.push((AssemblyInst::Raw { raw }, span));
    }

    /// Push a raw instruction.
    pub(crate) fn push_with_comment<C>(&mut self, raw: Inst, span: Span, comment: C)
    where
        C: AsRef<str>,
    {
        let pos = self.instructions.len();

        self.comments
            .entry(pos)
            .or_default()
            .push(comment.as_ref().to_owned());

        self.push(raw, span);
    }
}
