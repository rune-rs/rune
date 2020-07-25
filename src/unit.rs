use crate::collections::HashMap;
use crate::vm::{FnDynamicHash, Inst};
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
    pub instructions: Vec<Inst>,
    /// Where functions are located in the collection of instructions.
    functions: HashMap<FnDynamicHash, FnInfo>,
}

impl Unit {
    /// Construct a new unit.
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            functions: HashMap::new(),
        }
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
    ) -> Result<&mut Vec<Inst>, UnitError> {
        let offset = self.instructions.len();

        let hash = FnDynamicHash::of(name, args);

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

        Ok(&mut self.instructions)
    }
}
