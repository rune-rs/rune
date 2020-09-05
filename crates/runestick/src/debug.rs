use crate::unit::Label;
use crate::{Source, Span};

/// Debug information about a unit.
#[derive(Debug, Default)]
pub struct DebugInfo {
    /// File ids to source files.
    pub sources: Vec<Source>,
    /// Debug information on each instruction.
    pub instructions: Vec<DebugInst>,
}

impl DebugInfo {
    /// Get the source for the given source id.
    pub fn source_at(&self, source_id: usize) -> Option<&Source> {
        self.sources.get(source_id)
    }

    /// Get debug instruction at the given instruction pointer.
    pub fn instruction_at(&self, ip: usize) -> Option<&DebugInst> {
        self.instructions.get(ip)
    }

    /// Insert a source.
    pub fn insert_source(&mut self, source: Source) -> usize {
        let source_id = self.sources.len();
        self.sources.push(source);
        source_id
    }

    /// Iterate over all sources.
    pub fn sources(&self) -> impl Iterator<Item = (usize, &Source)> {
        self.sources.iter().enumerate()
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
