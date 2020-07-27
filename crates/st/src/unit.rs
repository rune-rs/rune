use crate::collections::HashMap;
use crate::hash::{FnDynamicHash, Hash};
use crate::vm::Inst;
use std::fmt;
use thiserror::Error;

/// Errors raised when building a new unit.
#[derive(Debug, Error)]
pub enum UnitError {
    /// Trying to register a conflicting function.
    #[error("conflicting function signature already exists `{existing}`")]
    FunctionConflict {
        /// The signature of an already existing function.
        existing: FnSignature,
    },
    /// A static string was missing for the given hash and slot.
    #[error("missing static string for hash `{hash}` and slot `{slot}`")]
    StaticStringMissing {
        /// The hash of the string.
        hash: Hash,
        /// The slot of the string.
        slot: usize,
    },
    /// A static string was missing for the given hash and slot.
    #[error("conflicting static string for hash `{hash}` between `{existing}` and `{string}`")]
    StaticStringHashConflict {
        /// The hash of the string.
        hash: Hash,
        /// The static string that was inserted.
        string: String,
        /// The existing static string that conflicted.
        existing: String,
    },
}

/// A description of a function signature.
#[derive(Debug)]
pub struct FnSignature {
    name: String,
    args: usize,
}

impl fmt::Display for FnSignature {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}(", self.name)?;

        let mut it = 0..self.args;
        let last = it.next_back();

        for _ in it {
            write!(fmt, "arg, ")?;
        }

        if last.is_some() {
            write!(fmt, "arg")?;
        }

        write!(fmt, ")")?;
        Ok(())
    }
}

/// Information about a registered function.
#[derive(Debug)]
pub struct FnInfo {
    /// Offset into the instruction set.
    offset: usize,
    /// Signature of the function.
    signature: FnSignature,
}

/// Instructions from a single source file.
#[derive(Debug)]
pub struct Unit {
    /// The instructions contained in the source file.
    instructions: Vec<Inst>,
    /// Where functions are located in the collection of instructions.
    functions: HashMap<FnDynamicHash, FnInfo>,
    /// A static string.
    static_strings: Vec<String>,
    /// Reverse lookup for static strings.
    static_string_rev: HashMap<Hash, usize>,
}

impl Unit {
    /// Construct a new unit.
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            functions: HashMap::new(),
            static_strings: Vec::new(),
            static_string_rev: HashMap::new(),
        }
    }

    /// Get the instruction at the given instruction pointer.
    pub fn instruction_at(&self, ip: usize) -> Option<Inst> {
        self.instructions.get(ip).copied()
    }

    /// Iterate over all static strings in the unit.
    pub fn iter_static_strings(&self) -> impl Iterator<Item = (Hash, &str)> + '_ {
        let mut it = self.static_strings.iter();

        std::iter::from_fn(move || {
            let s = it.next()?;
            Some((Hash::of(s), s.as_str()))
        })
    }

    /// Iterate over all instructions in order.
    pub fn iter_instructions(&self) -> impl Iterator<Item = Inst> + '_ {
        self.instructions.iter().copied()
    }

    /// Iterate over known functions.
    pub fn iter_functions(&self) -> impl Iterator<Item = (FnDynamicHash, &FnInfo)> + '_ {
        let mut it = self.functions.iter();

        std::iter::from_fn(move || {
            let (k, v) = it.next()?;
            Some((*k, v))
        })
    }

    /// Lookup the static string by slot, if it exists.
    pub fn lookup_string(&self, slot: usize) -> Option<&str> {
        self.static_strings.get(slot).map(String::as_str)
    }

    /// Insert a static string and return its associated slot that can later be
    /// looked up through [lookup_string][Self::lookup_string].
    ///
    /// Only uses up space if the static string is unique.
    pub fn static_string(&mut self, string: &str) -> Result<usize, UnitError> {
        let hash = Hash::of(string);

        if let Some(existing) = self.static_string_rev.get(&hash).copied() {
            let existing_string = self.static_strings.get(existing).ok_or_else(|| {
                UnitError::StaticStringMissing {
                    hash,
                    slot: existing,
                }
            })?;

            if existing_string != string {
                return Err(UnitError::StaticStringHashConflict {
                    hash,
                    string: string.to_owned(),
                    existing: existing_string.clone(),
                });
            }

            return Ok(existing);
        }

        let new_slot = self.static_strings.len();
        self.static_strings.push(string.to_owned());
        self.static_string_rev.insert(hash, new_slot);
        Ok(new_slot)
    }

    /// Lookup the location of a dynamic function.
    pub fn lookup(&self, hash: FnDynamicHash) -> Option<usize> {
        Some(self.functions.get(&hash)?.offset)
    }

    /// Construct a new function.
    pub fn new_function<'a>(
        &'a mut self,
        name: &str,
        args: usize,
        instructions: &[Inst],
    ) -> Result<(), UnitError> {
        let offset = self.instructions.len();

        let hash = Hash::of(name);
        let hash = FnDynamicHash::of(hash, args);

        let info = FnInfo {
            offset,
            signature: FnSignature {
                name: name.to_owned(),
                args,
            },
        };

        if let Some(old) = self.functions.insert(hash, info) {
            return Err(UnitError::FunctionConflict {
                existing: old.signature,
            });
        }

        self.instructions.extend(instructions.iter().copied());
        Ok(())
    }
}
