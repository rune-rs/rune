use crate::collections::HashMap;
use crate::functions::ItemPath;
use crate::hash::Hash;
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
        existing: UnitFnSignature,
    },
    /// Tried to add an import that conflicts with an existing one.
    #[error("conflicting import already exists `{existing}`")]
    ImportConflict {
        /// The signature of the old import.
        existing: ItemPath,
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

/// Information about a registered function.
#[derive(Debug)]
pub struct UnitFnInfo {
    /// Offset into the instruction set.
    offset: usize,
    /// Signature of the function.
    signature: UnitFnSignature,
}

/// A description of a function signature.
#[derive(Debug)]
pub struct UnitFnSignature {
    path: ItemPath,
    args: usize,
}

impl UnitFnSignature {
    /// Construct a new function signature.
    pub fn new(path: ItemPath, args: usize) -> Self {
        Self {
            path: path.to_owned(),
            args,
        }
    }
}

impl fmt::Display for UnitFnSignature {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}(", self.path)?;

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

/// Instructions from a single source file.
#[derive(Debug)]
pub struct Unit {
    /// The instructions contained in the source file.
    instructions: Vec<Inst>,
    /// All imports in the current unit.
    ///
    /// Only used to link against the current environment to make sure all
    /// required units are present.
    imports: HashMap<String, ItemPath>,
    /// Where functions are located in the collection of instructions.
    functions: HashMap<Hash, UnitFnInfo>,
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
            imports: HashMap::new(),
            functions: HashMap::new(),
            static_strings: Vec::new(),
            static_string_rev: HashMap::new(),
        }
    }

    /// Construct a new unit with the default prelude.
    pub fn with_default_prelude() -> Self {
        let mut this = Self::new();
        this.imports
            .insert(String::from("dbg"), ItemPath::of(&["core", "dbg"]));
        this
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
    pub fn iter_functions(&self) -> impl Iterator<Item = (Hash, &UnitFnInfo)> + '_ {
        let mut it = self.functions.iter();

        std::iter::from_fn(move || {
            let (k, v) = it.next()?;
            Some((*k, v))
        })
    }

    /// Iterate over known imports.
    pub fn iter_imports<'a>(&'a self) -> impl Iterator<Item = (&'a str, &'a ItemPath)> + '_ {
        let mut it = self.imports.iter();

        std::iter::from_fn(move || {
            let (k, v) = it.next()?;
            Some((k.as_str(), v))
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
    pub fn lookup(&self, hash: Hash) -> Option<usize> {
        Some(self.functions.get(&hash)?.offset)
    }

    /// Look up an import by name.
    pub fn lookup_import_by_name(&self, name: &str) -> Option<&ItemPath> {
        self.imports.get(name)
    }

    /// Declare a new import.
    pub fn new_import<I>(&mut self, path: I) -> Result<(), UnitError>
    where
        I: Copy + IntoIterator,
        I::Item: AsRef<str>,
    {
        let path = ItemPath::of(path);

        if let Some(last) = path.last() {
            if let Some(old) = self.imports.insert(last.to_owned(), path) {
                return Err(UnitError::ImportConflict { existing: old });
            }
        }

        Ok(())
    }

    /// Declare a new function at the current instruction pointer.
    pub fn new_function<I>(
        &mut self,
        path: I,
        args: usize,
        instructions: Vec<Inst>,
    ) -> Result<(), UnitError>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let offset = self.instructions.len();

        let path = ItemPath::of(path);
        let hash = Hash::function(&path);

        let info = UnitFnInfo {
            offset,
            signature: UnitFnSignature::new(path, args),
        };

        if let Some(old) = self.functions.insert(hash, info) {
            return Err(UnitError::FunctionConflict {
                existing: old.signature,
            });
        }

        self.instructions.extend(instructions);
        Ok(())
    }
}
