use core::fmt;

use crate as rune;
use crate::alloc;
use crate::alloc::prelude::*;
use crate::alloc::{HashMap, String, Vec};
use crate::ast;

/// A synthetic identifier which can be used to reference something in storage.
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[try_clone(copy)]
pub struct SyntheticId(usize);

impl fmt::Display for SyntheticId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04x}", self.0)
    }
}

/// The kind of a synthetic token.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum SyntheticKind {
    /// A synthetic label.
    Label,
    /// A synthetic string.
    String,
    /// A synthetic byte string.
    ByteString,
    /// A synthetic identifier,
    Ident,
    /// A synthetic number.
    Number,
}

impl fmt::Display for SyntheticKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyntheticKind::Label => "label".fmt(f),
            SyntheticKind::String => "string".fmt(f),
            SyntheticKind::ByteString => "byte string".fmt(f),
            SyntheticKind::Ident => "identifier".fmt(f),
            SyntheticKind::Number => "number".fmt(f),
        }
    }
}

/// Storage for synthetic language items.
#[derive(Default)]
pub(crate) struct Storage {
    /// Stored strings.
    strings: Vec<String>,
    /// Reverse lookup for existing strings.
    strings_rev: HashMap<String, SyntheticId>,
    /// Stored byte strings.
    byte_strings: Vec<Vec<u8>>,
    /// Reverse lookup for existing byte strings.
    byte_strings_rev: HashMap<Vec<u8>, SyntheticId>,
    /// Numbers stored.
    numbers: Vec<ast::Number>,
}

impl Storage {
    /// Construct a new number.
    ///
    /// The number will be stored in this storage, and will be synthetic
    /// (rather than from the source).
    pub(crate) fn insert_number<N>(&mut self, number: N) -> alloc::Result<SyntheticId>
    where
        ast::Number: From<N>,
    {
        let id = SyntheticId(self.numbers.len());
        self.numbers.try_push(number.into())?;
        Ok(id)
    }

    /// Insert the given text into storage and return its id.
    ///
    /// This will reuse old storage slots that already contains the given
    /// string.
    pub(crate) fn insert_str(&mut self, string: &str) -> alloc::Result<SyntheticId> {
        if let Some(id) = self.strings_rev.get(string).copied() {
            return Ok(id);
        }

        let id = SyntheticId(self.strings.len());
        let string = string.try_to_owned()?;
        self.strings.try_push(string.try_clone()?)?;
        self.strings_rev.try_insert(string, id)?;
        Ok(id)
    }

    /// Insert the given owned string into storage and return its id.
    ///
    /// This will reuse old storage slots that already contains the given
    /// string.
    pub(crate) fn insert_string(&mut self, string: String) -> alloc::Result<SyntheticId> {
        if let Some(id) = self.strings_rev.get(&string).copied() {
            return Ok(id);
        }

        let id = SyntheticId(self.strings.len());
        self.strings.try_push(string.try_clone()?)?;
        self.strings_rev.try_insert(string, id)?;
        Ok(id)
    }

    /// Insert the given text into storage and return its id.
    ///
    /// This will reuse old storage slots that already contains the given
    /// byte string.
    pub(crate) fn insert_byte_string(&mut self, bytes: &[u8]) -> alloc::Result<SyntheticId> {
        if let Some(id) = self.byte_strings_rev.get(bytes).copied() {
            return Ok(id);
        }

        let id = SyntheticId(self.byte_strings.len());
        self.byte_strings.try_push(Vec::try_from(bytes)?)?;
        self.byte_strings_rev
            .try_insert(Vec::try_from(bytes)?, id)?;
        Ok(id)
    }

    /// Get the content of the string with the specified id.
    pub(crate) fn get_string(&self, id: SyntheticId) -> Option<&str> {
        self.strings.get(id.0).map(|s| s.as_ref())
    }

    /// Get the content of the byte string with the specified id.
    pub(crate) fn get_byte_string(&self, id: SyntheticId) -> Option<&[u8]> {
        self.byte_strings.get(id.0).map(|b| b.as_ref())
    }

    /// Get the content of the number with the specified id.
    pub(crate) fn get_number(&self, id: SyntheticId) -> Option<&ast::Number> {
        self.numbers.get(id.0)
    }
}
