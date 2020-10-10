use crate::ast;
use crate::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;

/// Storage for synthetic language items.
#[derive(Clone, Default)]
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
    pub fn insert_number<N>(&self, number: N) -> usize
    where
        ast::Number: From<N>,
    {
        let mut inner = self.inner.borrow_mut();
        let id = inner.numbers.len();
        inner.numbers.push(number.into());
        id
    }

    /// Insert the given text into storage and return its id.
    ///
    /// This will reuse old storage slots that already contains the given
    /// string.
    pub fn insert_str(&self, string: &str) -> usize {
        let mut inner = self.inner.borrow_mut();

        if let Some(id) = inner.strings_rev.get(string).copied() {
            return id;
        }

        let id = inner.strings.len();
        let string = string.to_owned();
        inner.strings.push(string.clone());
        inner.strings_rev.insert(string, id);
        id
    }

    /// Insert the given owned string into storage and return its id.
    ///
    /// This will reuse old storage slots that already contains the given
    /// string.
    pub fn insert_string(&self, string: String) -> usize {
        let mut inner = self.inner.borrow_mut();

        if let Some(id) = inner.strings_rev.get(&string).copied() {
            return id;
        }

        let id = inner.strings.len();
        inner.strings.push(string.clone());
        inner.strings_rev.insert(string, id);
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

    /// Get the content of the byte string with the specified id and apply the
    /// given operation over it.
    pub fn with_byte_string<F, O>(&self, id: usize, with: F) -> Option<O>
    where
        F: FnOnce(&[u8]) -> O,
    {
        let inner = self.inner.borrow();
        let s = inner.byte_strings.get(id)?;
        Some(with(s))
    }

    /// Get the content of the string with the specified id.
    pub fn get_string(&self, id: usize) -> Option<String> {
        let inner = self.inner.borrow();
        inner.strings.get(id).cloned()
    }

    /// Get the content of the string with the specified id and apply the given
    /// operation over it.
    pub fn with_string<F, O>(&self, id: usize, with: F) -> Option<O>
    where
        F: FnOnce(&str) -> O,
    {
        let inner = self.inner.borrow();
        let s = inner.strings.get(id)?;
        Some(with(s))
    }

    /// Get the content of the byte string with the specified id.
    pub fn get_byte_string(&self, id: usize) -> Option<Vec<u8>> {
        let inner = self.inner.borrow();
        inner.byte_strings.get(id).cloned()
    }

    /// Get the content of the number with the specified id.
    pub fn get_number(&self, id: usize) -> Option<ast::Number> {
        let inner = self.inner.borrow();
        inner.numbers.get(id).cloned()
    }

    /// Get the content of the number with the specified id and apply the given
    /// operation over it.
    pub fn with_number<F, O>(&self, id: usize, with: F) -> Option<O>
    where
        F: FnOnce(&ast::Number) -> O,
    {
        let inner = self.inner.borrow();
        let s = inner.numbers.get(id)?;
        Some(with(s))
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
