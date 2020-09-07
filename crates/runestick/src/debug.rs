use crate::assembly::Label;
use crate::collections::HashMap;
use crate::{Hash, Item, Span};
use std::fmt;

/// Debug information about a unit.
#[derive(Debug, Default)]
pub struct DebugInfo {
    /// Debug information on each instruction.
    pub instructions: Vec<DebugInst>,
    /// Function signatures.
    pub functions: HashMap<Hash, DebugSignature>,
    /// Reverse lookup of a function.
    pub functions_rev: HashMap<usize, Hash>,
}

impl DebugInfo {
    /// Get debug instruction at the given instruction pointer.
    pub fn instruction_at(&self, ip: usize) -> Option<&DebugInst> {
        self.instructions.get(ip)
    }

    /// Get the function corresponding to the given instruction pointer.
    pub fn function_at(&self, ip: usize) -> Option<(Hash, &DebugSignature)> {
        let hash = *self.functions_rev.get(&ip)?;
        let signature = self.functions.get(&hash)?;
        Some((hash, signature))
    }
}

/// Debug information for every instruction.
#[derive(Debug)]
pub struct DebugInst {
    /// The file by id the instruction belongs to.
    pub source_id: usize,
    /// The span of the instruction.
    pub span: Span,
    /// The comment for the line.
    pub comment: Option<String>,
    /// Label associated with the location.
    pub label: Option<Label>,
}

/// A description of a function signature.
#[derive(Debug, Clone)]
pub struct DebugSignature {
    /// The path of the function.
    pub path: Item,
    /// The number of arguments expected in the function.
    pub args: usize,
}

impl DebugSignature {
    /// Construct a new function signature.
    pub fn new(path: Item, args: usize) -> Self {
        Self { path, args }
    }
}

impl fmt::Display for DebugSignature {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}(", self.path)?;

        let mut it = 0..self.args;
        let last = it.next_back();

        for _ in it {
            write!(fmt, "arg, ")?;
        }

        if last.is_some() {
            write!(fmt, "arg")?;
        }

        write!(fmt, ")")?;
        Ok(())
    }
}
