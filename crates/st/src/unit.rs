//! A single execution unit in the ST virtual machine.
//!
//! A unit consists of an array of instructions, and lookaside tables for
//! metadata like function locations.

use crate::collections::HashMap;
use crate::context::Item;
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
        existing: Item,
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
    /// Tried to add a duplicate label.
    #[error("duplicate label `{label}`")]
    DuplicateLabel {
        /// The duplicate label.
        label: Label,
    },
    /// The specified label is missing.
    #[error("missing label `{label}`")]
    MissingLabel {
        /// The missing label.
        label: Label,
    },
}

/// A span corresponding to a range in the source file being parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Span {
    /// The start of the span in bytes.
    pub start: usize,
    /// The end of the span in bytes.
    pub end: usize,
}

impl Span {
    /// Join this span with another span.
    pub fn join(self, other: Self) -> Self {
        Self {
            start: usize::min(self.start, other.start),
            end: usize::min(self.end, other.end),
        }
    }

    /// Get the point span.
    pub fn point(pos: usize) -> Self {
        Self {
            start: pos,
            end: pos,
        }
    }

    /// Narrow the span with the given amount.
    pub fn narrow(self, amount: usize) -> Self {
        Self {
            start: self.start.saturating_add(amount),
            end: self.end.saturating_sub(amount),
        }
    }

    /// Return the zero-based line and column.
    pub fn line_col(self, source: &str) -> (usize, usize) {
        let mut line = 0;
        let mut col = 0;

        let mut it = source.chars();
        let mut count = 0;

        while let Some(c) = it.next() {
            if count >= self.start {
                break;
            }

            count += c.encode_utf8(&mut [0u8; 4]).len();

            if let '\n' | '\r' = c {
                if c == '\n' {
                    line += 1;
                }

                if col > 0 {
                    col = 0;
                }

                continue;
            }

            col += 1;
        }

        (line, col)
    }
}

/// Information about a registered function.
#[derive(Debug)]
pub struct UnitFnInfo {
    /// Offset into the instruction set.
    pub offset: usize,
    /// Signature of the function.
    pub signature: UnitFnSignature,
}

/// A description of a function signature.
#[derive(Debug)]
pub struct UnitFnSignature {
    path: Item,
    args: usize,
}

impl UnitFnSignature {
    /// Construct a new function signature.
    pub fn new(path: Item, args: usize) -> Self {
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

/// Debug information for every instruction.
#[derive(Debug)]
pub struct DebugInfo {
    /// The span of the instruction.
    pub span: Span,
    /// The comment for the line.
    pub comment: Option<Box<str>>,
    /// Label associated with the location.
    pub label: Option<Label>,
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
    imports: HashMap<String, Item>,
    /// Where functions are located in the collection of instructions.
    functions: HashMap<Hash, UnitFnInfo>,
    /// Function by address.
    functions_rev: HashMap<usize, Hash>,
    /// A static string.
    static_strings: Vec<String>,
    /// Reverse lookup for static strings.
    static_string_rev: HashMap<Hash, usize>,
    /// Debug info for each line.
    debug: Vec<DebugInfo>,
    /// The current label count.
    label_count: usize,
}

impl Unit {
    /// Construct a new unit.
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            imports: HashMap::new(),
            functions: HashMap::new(),
            functions_rev: HashMap::new(),
            static_strings: Vec::new(),
            static_string_rev: HashMap::new(),
            debug: Vec::new(),
            label_count: 0,
        }
    }

    /// Construct a new unit with the default prelude.
    pub fn with_default_prelude() -> Self {
        let mut this = Self::new();
        this.imports
            .insert(String::from("dbg"), Item::of(&["core", "dbg"]));
        this.imports
            .insert(String::from("unit"), Item::of(&["core", "unit"]));
        this.imports
            .insert(String::from("bool"), Item::of(&["core", "bool"]));
        this.imports
            .insert(String::from("char"), Item::of(&["core", "char"]));
        this.imports
            .insert(String::from("int"), Item::of(&["core", "int"]));
        this.imports
            .insert(String::from("float"), Item::of(&["core", "float"]));
        this.imports
            .insert(String::from("Object"), Item::of(&["core", "Object"]));
        this.imports
            .insert(String::from("Array"), Item::of(&["core", "Array"]));
        this.imports.insert(
            String::from("String"),
            Item::of(&["std", "string", "String"]),
        );
        this
    }

    /// Access the function at the given instruction location.
    pub fn function_at(&self, n: usize) -> Option<(Hash, &UnitFnInfo)> {
        let hash = self.functions_rev.get(&n).copied()?;
        Some((hash, self.functions.get(&hash)?))
    }

    /// Access debug information for the given location if it is available.
    pub fn debug_info_at(&self, n: usize) -> Option<&DebugInfo> {
        self.debug.get(n)
    }

    /// Get the instruction at the given instruction pointer.
    pub fn instruction_at(&self, ip: usize) -> Option<&Inst> {
        self.instructions.get(ip)
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
    pub fn iter_imports<'a>(&'a self) -> impl Iterator<Item = (&'a str, &'a Item)> + '_ {
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
    pub fn lookup_import_by_name(&self, name: &str) -> Option<&Item> {
        self.imports.get(name)
    }

    /// Declare a new import.
    pub fn new_import<I>(&mut self, path: I) -> Result<(), UnitError>
    where
        I: Copy + IntoIterator,
        I::Item: AsRef<str>,
    {
        let path = Item::of(path);

        if let Some(last) = path.last() {
            if let Some(old) = self.imports.insert(last.to_owned(), path) {
                return Err(UnitError::ImportConflict { existing: old });
            }
        }

        Ok(())
    }

    /// Construct a new empty assembly associated with the current unit.
    pub fn new_assembly(&mut self) -> Assembly {
        Assembly::new(self.label_count)
    }

    /// Declare a new function at the current instruction pointer.
    pub fn new_function<I>(
        &mut self,
        path: I,
        args: usize,
        assembly: Assembly,
    ) -> Result<(), UnitError>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let offset = self.instructions.len();
        let path = Item::of(path);
        let hash = Hash::function(&path);

        self.functions_rev.insert(offset, hash);

        let info = UnitFnInfo {
            offset,
            signature: UnitFnSignature::new(path, args),
        };

        if let Some(old) = self.functions.insert(hash, info) {
            return Err(UnitError::FunctionConflict {
                existing: old.signature,
            });
        }

        self.add_assembly(assembly)?;
        Ok(())
    }

    /// Translate the given assembly into instructions.
    fn add_assembly(&mut self, assembly: Assembly) -> Result<(), UnitError> {
        self.label_count = assembly.label_count;

        for (pos, (inst, span)) in assembly.instructions.into_iter().enumerate() {
            let mut comment = None;
            let label = assembly.labels_rev.get(&pos).copied();

            self.instructions.push(match inst {
                AssemblyInst::Jump { label } => {
                    comment = Some(format!("label:{}", label).into_boxed_str());

                    Inst::Jump {
                        offset: translate_offset(pos, label, &assembly.labels)?,
                    }
                }
                AssemblyInst::JumpIf { label } => {
                    comment = Some(format!("label:{}", label).into_boxed_str());

                    Inst::JumpIf {
                        offset: translate_offset(pos, label, &assembly.labels)?,
                    }
                }
                AssemblyInst::JumpIfNot { label } => {
                    comment = Some(format!("label:{}", label).into_boxed_str());

                    Inst::JumpIfNot {
                        offset: translate_offset(pos, label, &assembly.labels)?,
                    }
                }
                AssemblyInst::Raw { raw } => raw,
            });

            self.debug.push(DebugInfo {
                span,
                comment,
                label,
            });
        }

        return Ok(());

        fn translate_offset(
            base: usize,
            label: Label,
            labels: &HashMap<Label, usize>,
        ) -> Result<isize, UnitError> {
            let base = base as isize;

            let offset = labels
                .get(&label)
                .copied()
                .ok_or_else(|| UnitError::MissingLabel {
                    label: label.to_owned(),
                })?;

            Ok((offset as isize) - base)
        }
    }
}

/// A label that can be jumped to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Label {
    name: &'static str,
    ident: usize,
}

impl fmt::Display for Label {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}_{}", self.name, self.ident)
    }
}

#[derive(Debug, Clone)]
enum AssemblyInst {
    Jump { label: Label },
    JumpIf { label: Label },
    JumpIfNot { label: Label },
    Raw { raw: Inst },
}

/// Helper structure to build instructions and maintain certain invariants.
#[derive(Debug, Clone, Default)]
pub struct Assembly {
    /// Label to offset.
    labels: HashMap<Label, usize>,
    /// Registered label by offset.
    labels_rev: HashMap<usize, Label>,
    /// Instructions with spans.
    instructions: Vec<(AssemblyInst, Span)>,
    /// The number of labels.
    label_count: usize,
}

impl Assembly {
    /// Construct a new assembly.
    fn new(label_count: usize) -> Self {
        Self {
            labels: Default::default(),
            labels_rev: Default::default(),
            instructions: Default::default(),
            label_count,
        }
    }

    /// Construct and return a new label.
    pub fn new_label(&mut self, name: &'static str) -> Label {
        let label = Label {
            name,
            ident: self.label_count,
        };

        self.label_count += 1;
        label
    }

    /// Apply the label at the current instruction offset.
    pub fn label(&mut self, label: Label) -> Result<Label, UnitError> {
        let offset = self.instructions.len();

        if let Some(_) = self.labels.insert(label, offset) {
            return Err(UnitError::DuplicateLabel { label });
        }

        self.labels_rev.insert(offset, label);
        Ok(label)
    }

    /// Add a jump to the given label.
    pub fn jump(&mut self, label: Label, span: Span) {
        self.instructions.push((AssemblyInst::Jump { label }, span));
    }

    /// Add a conditional jump to the given label.
    pub fn jump_if(&mut self, label: Label, span: Span) {
        self.instructions
            .push((AssemblyInst::JumpIf { label }, span));
    }

    /// Add a conditional jump to the given label.
    pub fn jump_if_not(&mut self, label: Label, span: Span) {
        self.instructions
            .push((AssemblyInst::JumpIfNot { label }, span));
    }

    /// Push a raw instruction.
    pub fn push(&mut self, raw: Inst, span: Span) {
        self.instructions.push((AssemblyInst::Raw { raw }, span));
    }
}
