use crate::global::Global;
use crate::{Block, Var};
use std::fmt;

/// The central state machine assembler.
pub struct Program {
    global: Global,
    blocks: Vec<Block>,
}

impl Program {
    /// Construct a new empty state machine.
    pub fn new() -> Self {
        Self {
            global: Global::default(),
            blocks: Vec::new(),
        }
    }

    /// Allocate a new value.
    pub fn var(&self) -> Var {
        self.global.var()
    }

    /// Construct a new block associated with the state machine.
    pub fn block(&mut self) -> Block {
        let block = self.global.block(None);
        self.blocks.push(block.clone());
        block
    }

    /// Construct a block with a name.
    pub fn named(&mut self, name: &str) -> Block {
        let block = self.global.block(Some(name.into()));
        self.blocks.push(block.clone());
        block
    }

    /// Dump the current state of the program.
    ///
    /// This is useful for diagnostics.
    pub fn dump(&self) -> ProgramDump<'_> {
        ProgramDump(self)
    }
}

pub struct ProgramDump<'a>(&'a Program);

impl fmt::Display for ProgramDump<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let constants = self.0.global.constants();

        if !constants.is_empty() {
            writeln!(f, "constants:")?;

            for (id, c) in constants.iter().enumerate() {
                writeln!(f, "  C{} <- {:?}", id, c)?;
            }

            if !self.0.blocks.is_empty() {
                writeln!(f)?;
            }
        }

        let mut it = self.0.blocks.iter();
        let last = it.next_back();

        for b in it {
            writeln!(f, "{}", b.dump())?;
        }

        if let Some(b) = last {
            write!(f, "{}", b.dump())?;
        }

        Ok(())
    }
}
