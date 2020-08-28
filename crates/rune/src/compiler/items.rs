use crate::error::CompileError;
use runestick::{Component, Item, Span};

pub(super) struct Guard(usize);

/// Manage item paths.
pub(super) struct Items {
    block_count: usize,
    path: Vec<Component>,
}

impl Items {
    /// Construct a new items manager.
    pub fn new(base: Vec<Component>) -> Self {
        Self {
            block_count: 0,
            path: base,
        }
    }

    /// Push a component and return a guard to it.
    pub fn push_block(&mut self) -> Guard {
        let guard = Guard(self.path.len());
        self.path.push(Component::Block(self.block_count));
        self.block_count += 1;
        guard
    }

    /// Push a component and return a guard to it.
    pub fn push_name(&mut self, name: &str) -> Guard {
        let guard = Guard(self.path.len());
        self.path.push(Component::String(name.to_owned()));
        guard
    }

    /// Pop the last component.
    pub fn pop(&mut self, Guard(index): Guard, span: Span) -> Result<(), CompileError> {
        let last = self
            .path
            .pop()
            .ok_or_else(|| CompileError::internal("no item on stack", span))?;

        if self.path.len() != index {
            return Err(CompileError::internal("item stack mismatch", span));
        }

        if let Component::Block(..) = last {
            self.block_count = self
                .block_count
                .checked_sub(1)
                .ok_or_else(|| CompileError::internal("block count overflow", span))?;
        }

        Ok(())
    }

    /// Get the item for the current state of the path.
    pub fn item(&self) -> Item {
        Item::of(&self.path)
    }
}
