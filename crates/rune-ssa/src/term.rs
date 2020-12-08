use crate::{Assign, BlockId};
use std::fmt;

/// An operation that terminates a block.
pub enum Term {
    /// Default termination. The procedure will panic.
    Panic,
    /// Conditionally jump to the given block if the given condition is true.
    JumpIf {
        /// The condition of the jump.
        condition: Assign,
        /// Where to jump if the condition is true.
        then_block: BlockId,
        /// Where to jump if the condition is false.
        else_block: BlockId,
    },
    /// Unconditionally jump to the given block.
    Jump {
        /// Block to jump to.
        block: BlockId,
    },
    /// Return from the current procedure with the given value.
    Return {
        /// The value to return.
        var: Assign,
    },
}

impl Term {
    /// Dump the block terminator diagnostically.
    pub(crate) fn dump(&self) -> TermDump<'_> {
        TermDump { term: self }
    }
}

pub(crate) struct TermDump<'a> {
    term: &'a Term,
}

impl fmt::Display for TermDump<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.term {
            Term::Panic => {
                write!(f, "panic")?;
            }
            Term::JumpIf {
                condition,
                then_block,
                else_block,
            } => {
                write!(f, "jump-if {}, {}, {}", condition, then_block, else_block)?;
            }
            Term::Jump { block } => {
                write!(f, "jump {}", block)?;
            }
            Term::Return { var } => {
                write!(f, "return {}", var)?;
            }
        }

        Ok(())
    }
}
