use crate::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;

/// Storage for synthetic language items.
#[derive(Clone)]
pub struct Storage {
    inner: Rc<RefCell<Inner>>,
}

impl Storage {
    pub(crate) fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(Inner {
                idents: Vec::new(),
                idents_rev: HashMap::new(),
            })),
        }
    }

    /// Insert the given identifier as a synthetic identifier.
    pub fn insert_ident(&self, ident: &str) -> usize {
        let mut inner = self.inner.borrow_mut();

        if let Some(id) = inner.idents_rev.get(ident).copied() {
            return id;
        }

        let id = inner.idents.len();
        inner.idents.push(ident.to_string());
        inner.idents_rev.insert(ident.to_string(), id);
        id
    }

    /// Get the content of the identifier with the specified id.
    pub fn get_ident(&self, id: usize) -> Option<String> {
        let inner = self.inner.borrow();
        inner.idents.get(id).cloned()
    }
}

struct Inner {
    /// Identifiers stored.
    idents: Vec<String>,
    /// Reverse lookup for existing identifiers.
    idents_rev: HashMap<String, usize>,
}
