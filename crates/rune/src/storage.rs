use crate::ast;
use crate::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;

/// Storage for synthetic language items.
#[derive(Default, Clone)]
pub struct Storage {
    inner: Rc<RefCell<Inner>>,
}

impl Storage {
    /// Construct a new empty storage.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Construct a new number.
    ///
    /// The number will be stored in this storage, and will be synthetic
    /// (rather than from the source).
    pub fn insert_number<N>(&self, number: N) -> ast::Kind
    where
        ast::Number: From<N>,
    {
        let mut inner = self.inner.borrow_mut();
        let id = inner.numbers.len();
        inner.numbers.push(number.into());
        ast::Kind::LitNumber(ast::NumberSource::Synthetic(id))
    }

    /// Insert the given text into storage and return its id.
    ///
    /// This will reuse old storage slots that already contains the given
    /// string.
    pub fn insert_string(&self, string: &str) -> usize {
        let mut inner = self.inner.borrow_mut();

        if let Some(id) = inner.strings_rev.get(string).copied() {
            return id;
        }

        let id = inner.strings.len();
        inner.strings.push(string.to_string());
        inner.strings_rev.insert(string.to_string(), id);
        id
    }

    /// Insert the given text into storage and return its id.
    ///
    /// This will reuse old storage slots that already contains the given
    /// byte string.
    pub fn insert_byte_string(&self, bytes: &[u8]) -> usize {
        let mut inner = self.inner.borrow_mut();

        if let Some(id) = inner.byte_strings_rev.get(bytes).copied() {
            return id;
        }

        let id = inner.byte_strings.len();
        inner.byte_strings.push(bytes.to_vec());
        inner.byte_strings_rev.insert(bytes.to_vec(), id);
        id
    }

    /// Get the content of the string with the specified id.
    pub fn get_string(&self, id: usize) -> Option<String> {
        let inner = self.inner.borrow();
        inner.strings.get(id).cloned()
    }

    /// Get the content of the byte string with the specified id.
    pub fn get_byte_string(&self, id: usize) -> Option<Vec<u8>> {
        let inner = self.inner.borrow();
        inner.byte_strings.get(id).cloned()
    }

    /// Get the content of the number with the specified id.
    pub fn get_number(&self, id: usize) -> Option<ast::Number> {
        let inner = self.inner.borrow();
        inner.numbers.get(id).copied()
    }
}

#[derive(Default)]
struct Inner {
    /// Stored strings.
    strings: Vec<String>,
    /// Reverse lookup for existing strings.
    strings_rev: HashMap<String, usize>,
    /// Stored byte strings.
    byte_strings: Vec<Vec<u8>>,
    /// Reverse lookup for existing byte strings.
    byte_strings_rev: HashMap<Vec<u8>, usize>,
    /// Numbers stored.
    numbers: Vec<ast::Number>,
}
