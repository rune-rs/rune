use crate::ast;
use crate::collections::HashMap;

/// Storage for synthetic language items.
#[derive(Default)]
pub struct Storage {
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

impl Storage {
    /// Construct a new empty storage.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Construct a new number.
    ///
    /// The number will be stored in this storage, and will be synthetic
    /// (rather than from the source).
    pub fn insert_number<N>(&mut self, number: N) -> usize
    where
        ast::Number: From<N>,
    {
        let id = self.numbers.len();
        self.numbers.push(number.into());
        id
    }

    /// Insert the given text into storage and return its id.
    ///
    /// This will reuse old storage slots that already contains the given
    /// string.
    pub fn insert_str(&mut self, string: &str) -> usize {
        if let Some(id) = self.strings_rev.get(string).copied() {
            return id;
        }

        let id = self.strings.len();
        let string = string.to_owned();
        self.strings.push(string.clone());
        self.strings_rev.insert(string, id);
        id
    }

    /// Insert the given owned string into storage and return its id.
    ///
    /// This will reuse old storage slots that already contains the given
    /// string.
    pub fn insert_string(&mut self, string: String) -> usize {
        if let Some(id) = self.strings_rev.get(&string).copied() {
            return id;
        }

        let id = self.strings.len();
        self.strings.push(string.clone());
        self.strings_rev.insert(string, id);
        id
    }

    /// Insert the given text into storage and return its id.
    ///
    /// This will reuse old storage slots that already contains the given
    /// byte string.
    pub fn insert_byte_string(&mut self, bytes: &[u8]) -> usize {
        if let Some(id) = self.byte_strings_rev.get(bytes).copied() {
            return id;
        }

        let id = self.byte_strings.len();
        self.byte_strings.push(bytes.to_vec());
        self.byte_strings_rev.insert(bytes.to_vec(), id);
        id
    }

    /// Get the content of the string with the specified id.
    pub fn get_string(&self, id: usize) -> Option<&String> {
        self.strings.get(id)
    }

    /// Get the content of the byte string with the specified id.
    pub fn get_byte_string(&self, id: usize) -> Option<&Vec<u8>> {
        self.byte_strings.get(id)
    }

    /// Get the content of the number with the specified id.
    pub fn get_number(&self, id: usize) -> Option<&ast::Number> {
        self.numbers.get(id)
    }
}
