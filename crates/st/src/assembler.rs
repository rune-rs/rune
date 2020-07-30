use crate::collections::HashMap;
use crate::vm;
use thiserror::Error;

/// Error raised during assembly of instructions.
#[derive(Debug, Error)]
pub enum AssemblyError {
    /// The specified label is missing.
    #[error("missing label `{label}`")]
    MissingLabel {
        /// The missing label.
        label: String,
    },
    /// Tried to add a duplicate label.
    #[error("duplicate label `{label}`")]
    DuplicateLabel {
        /// The duplicate label.
        label: String,
    },
}

#[derive(Debug, Clone)]
enum Inst {
    Jump { label: String },
    JumpIf { label: String },
    JumpIfNot { label: String },
    Raw { raw: vm::Inst },
}

/// Helper structure to build instructions and maintain certain invariants.
#[derive(Debug, Clone, Default)]
pub struct Assembler {
    labels: HashMap<String, usize>,
    instructions: Vec<Inst>,
}

impl Assembler {
    /// Construct a new assembler.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add the given label att the current instruction offset.
    pub fn label<S>(&mut self, label: S) -> Result<(), AssemblyError>
    where
        S: AsRef<str>,
    {
        let offset = self.instructions.len();
        let label = label.as_ref();

        if let Some(_) = self.labels.insert(label.to_owned(), offset) {
            return Err(AssemblyError::DuplicateLabel {
                label: label.to_owned(),
            });
        }

        Ok(())
    }

    /// Add a jump to the given label.
    pub fn jump<S>(&mut self, label: S)
    where
        S: AsRef<str>,
    {
        self.instructions.push(Inst::Jump {
            label: label.as_ref().to_owned(),
        });
    }

    /// Add a conditional jump to the given label.
    pub fn jump_if<S>(&mut self, label: S)
    where
        S: AsRef<str>,
    {
        self.instructions.push(Inst::JumpIf {
            label: label.as_ref().to_owned(),
        });
    }

    /// Add a conditional jump to the given label.
    pub fn jump_if_not<S>(&mut self, label: S)
    where
        S: AsRef<str>,
    {
        self.instructions.push(Inst::JumpIfNot {
            label: label.as_ref().to_owned(),
        });
    }

    /// Push a raw instruction.
    pub fn push(&mut self, raw: vm::Inst) {
        self.instructions.push(Inst::Raw { raw });
    }

    /// Translate the given assembly into instructions.
    pub fn assembly(self) -> Result<Vec<vm::Inst>, AssemblyError> {
        let mut output = Vec::with_capacity(self.instructions.len());

        for (base, inst) in self.instructions.into_iter().enumerate() {
            output.push(match inst {
                Inst::Jump { label } => vm::Inst::Jump {
                    offset: translate_offset(base, &label, &self.labels)?,
                },
                Inst::JumpIf { label } => vm::Inst::JumpIf {
                    offset: translate_offset(base, &label, &self.labels)?,
                },
                Inst::JumpIfNot { label } => vm::Inst::JumpIfNot {
                    offset: translate_offset(base, &label, &self.labels)?,
                },
                Inst::Raw { raw } => raw,
            });
        }

        return Ok(output);

        fn translate_offset(
            base: usize,
            label: &str,
            labels: &HashMap<String, usize>,
        ) -> Result<isize, AssemblyError> {
            let base = base as isize;
            let offset = labels
                .get(label)
                .ok_or_else(|| AssemblyError::MissingLabel {
                    label: label.to_owned(),
                })?;
            Ok((*offset as isize) - base)
        }
    }
}
