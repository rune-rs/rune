use runestick::{Component, Item};
use std::cell::RefCell;
use std::mem;
use std::rc::Rc;

pub(super) struct Guard {
    path: Rc<RefCell<Vec<Node>>>,
}

impl Drop for Guard {
    fn drop(&mut self) {
        let exists = self.path.borrow_mut().pop().is_some();
        debug_assert!(exists);
    }
}

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
    path: Rc<RefCell<Vec<Node>>>,
}

impl Items {
    /// Construct a new items manager.
    pub fn new(base: Vec<Component>) -> Self {
        let path = base
            .into_iter()
            .map(|component| Node {
                blocks: 0,
                closures: 0,
                component,
            })
            .collect();

        Self {
            path: Rc::new(RefCell::new(path)),
        }
    }

    /// Check if the current path is empty.
    pub fn is_empty(&self) -> bool {
        self.path.borrow().is_empty()
    }

    /// Get the next block index.
    fn next_block(&mut self) -> usize {
        let mut path = self.path.borrow_mut();

        if let Some(node) = path.last_mut() {
            let new = node.blocks + 1;
            mem::replace(&mut node.blocks, new)
        } else {
            0
        }
    }

    /// Get the next closure index.
    fn next_closure(&mut self) -> usize {
        let mut path = self.path.borrow_mut();

        if let Some(node) = path.last_mut() {
            let new = node.closures + 1;
            mem::replace(&mut node.closures, new)
        } else {
            0
        }
    }

    /// Push a component and return a guard to it.
    pub fn push_block(&mut self) -> Guard {
        let index = self.next_block();
        self.path
            .borrow_mut()
            .push(Node::from(Component::Block(index)));

        Guard {
            path: self.path.clone(),
        }
    }

    /// Push a closure component and return guard associated with it.
    pub fn push_closure(&mut self) -> Guard {
        let index = self.next_closure();
        self.path
            .borrow_mut()
            .push(Node::from(Component::Closure(index)));

        Guard {
            path: self.path.clone(),
        }
    }

    /// Push a component and return a guard to it.
    pub fn push_name(&mut self, name: &str) -> Guard {
        self.path
            .borrow_mut()
            .push(Node::from(Component::String(name.to_owned())));

        Guard {
            path: self.path.clone(),
        }
    }

    /// Get the item for the current state of the path.
    pub fn item(&self) -> Item {
        let path = self.path.borrow();
        Item::of(path.iter().map(|n| &n.component))
    }
}
