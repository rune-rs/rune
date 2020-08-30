use crate::error::CompileError;
use runestick::{Component, Item, Span};
use std::mem;

pub(super) struct Guard(usize);

struct Node {
    blocks: usize,
    closures: usize,
    component: Component,
}

impl From<Component> for Node {
    fn from(component: Component) -> Self {
        Self {
            blocks: 0,
            closures: 0,
            component,
        }
    }
}

/// Manage item paths.
pub(super) struct Items {
    path: Vec<Node>,
}

impl Items {
    /// Construct a new items manager.
    pub fn new(base: Vec<Component>) -> Self {
        Self {
            path: base
                .into_iter()
                .map(|component| Node {
                    blocks: 0,
                    closures: 0,
                    component,
                })
                .collect(),
        }
    }

    /// Check if the current path is empty.
    pub fn is_empty(&self) -> bool {
        self.path.is_empty()
    }

    /// Get the next block index.
    fn next_block(&mut self) -> usize {
        if let Some(node) = self.path.last_mut() {
            let new = node.blocks + 1;
            mem::replace(&mut node.blocks, new)
        } else {
            0
        }
    }

    /// Get the next closure index.
    fn next_closure(&mut self) -> usize {
        if let Some(node) = self.path.last_mut() {
            let new = node.closures + 1;
            mem::replace(&mut node.closures, new)
        } else {
            0
        }
    }

    /// Push a component and return a guard to it.
    pub fn push_block(&mut self) -> Guard {
        let index = self.next_block();
        let guard = Guard(self.path.len());
        self.path.push(Node::from(Component::Block(index)));
        guard
    }

    /// Push a closure component and return guard associated with it.
    pub fn push_closure(&mut self) -> Guard {
        let index = self.next_closure();
        let guard = Guard(self.path.len());
        self.path.push(Node::from(Component::Closure(index)));
        guard
    }

    /// Push a component and return a guard to it.
    pub fn push_name(&mut self, name: &str) -> Guard {
        let guard = Guard(self.path.len());
        self.path
            .push(Node::from(Component::String(name.to_owned())));
        guard
    }

    /// Pop the last component.
    pub fn pop(&mut self, Guard(guard): Guard, span: Span) -> Result<(), CompileError> {
        self.path
            .pop()
            .ok_or_else(|| CompileError::internal("no item on stack", span))?;

        if self.path.len() != guard {
            return Err(CompileError::internal("item stack mismatch", span));
        }

        Ok(())
    }

    /// Get the item for the current state of the path.
    pub fn item(&self) -> Item {
        Item::of(self.path.iter().map(|n| &n.component))
    }
}
