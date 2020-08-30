use crate::error::CompileError;
use runestick::{Component, Item, Span};
use std::mem;

pub(super) struct Guard(usize);

struct Node {
    children: usize,
    component: Component,
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
                    children: 0,
                    component,
                })
                .collect(),
        }
    }

    /// Check if the current path is empty.
    pub fn is_empty(&self) -> bool {
        self.path.is_empty()
    }

    /// Get the next child index.
    fn next_index(&mut self) -> usize {
        if let Some(node) = self.path.last_mut() {
            let new = node.children + 1;
            mem::replace(&mut node.children, new)
        } else {
            0
        }
    }

    /// Push a component and return a guard to it.
    pub fn push_block(&mut self) -> Guard {
        let index = self.next_index();
        let guard = Guard(self.path.len());

        self.path.push(Node {
            children: 0,
            component: Component::Block(index),
        });

        guard
    }

    /// Push a component and return a guard to it.
    pub fn push_name(&mut self, name: &str) -> Guard {
        let guard = Guard(self.path.len());

        self.path.push(Node {
            children: 0,
            component: Component::String(name.to_owned()),
        });

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
