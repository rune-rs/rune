use crate::assembly::Label;
use crate::Span;

/// Debug information about a unit.
#[derive(Debug, Default)]
pub struct DebugInfo {
    /// Debug information on each instruction.
    pub instructions: Vec<DebugInst>,
}

impl DebugInfo {
    /// Get debug instruction at the given instruction pointer.
    pub fn instruction_at(&self, ip: usize) -> Option<&DebugInst> {
        self.instructions.get(ip)
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
