//! The state machine assembler of Rune.

use crate::collections::HashMap;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;
use thiserror::Error;

/// Error raised during machine construction.
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum Error {
    #[error("mismatch in block inputs, expected {expected} but got {actual}")]
    BlockInputMismatch { expected: usize, actual: usize },
}

/// A constant value.
#[derive(Debug)]
pub enum Constant {
    /// The unit constant (always has constant id = 0).
    Unit,
    /// A boolean constant.
    Bool(bool),
    /// A character constant.
    Char(char),
    /// A byte constant.
    Byte(u8),
    /// An integer constant.
    Integer(i64),
    /// A float constant.
    Float(f64),
    /// A string constant.
    String(Box<str>),
    /// A byte constant.
    Bytes(Box<[u8]>),
}

/// The identifier of a constant.
#[derive(Debug, Clone, Copy)]
pub struct ConstId(usize);

impl fmt::Display for ConstId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A single abstract machine instruction.
pub enum Inst {
    /// An instruction to load a constant as a value.
    Const(ConstId),
    /// Add two values together.
    Add(ValueId, ValueId),
    /// Conditionally jump to the given block if the given condition is true.
    JumpIf(ValueId, BlockId, Vec<ValueId>),
    /// Return from the current procedure with the given value.
    Return(ValueId),
}

/// A variable that can be used as block entries or temporaries.
/// Instructions typically produce and use vars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValueId(usize);

impl fmt::Display for ValueId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Identifier to a block.
#[derive(Debug, Clone, Copy)]
pub struct BlockId(usize);

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A block containing a sequence of assignments.
///
/// A block carries a definition of its entry.
/// The entry is the sequence of input variables the block expects.
pub struct Block {
    inner: Rc<BlockInner>,
}

impl Block {
    /// Construct a new empty block.
    fn new(global: Global) -> Self {
        let id = global.block();

        Self {
            inner: Rc::new(BlockInner {
                id,
                global,
                entry: RefCell::new(Vec::new()),
                assignments: RefCell::new(HashMap::new()),
                instructions: RefCell::new(Vec::new()),
            }),
        }
    }

    /// Get the identifier of the block.
    pub fn id(&self) -> BlockId {
        self.inner.id
    }

    /// Allocate an entry variable.
    pub fn entry(&mut self) -> ValueId {
        let value = self.inner.global.value();
        self.inner.entry.borrow_mut().push(value);
        value
    }

    /// Load a constant as a variable.
    pub fn constant(&mut self, constant: Constant) -> ValueId {
        let value = self.inner.global.value();
        let const_id = self.inner.global.constant(constant);
        self.inner
            .assignments
            .borrow_mut()
            .insert(value, Inst::Const(const_id));
        value
    }

    /// Add two variables together and produce the resulting variable.
    pub fn add(&mut self, a: ValueId, b: ValueId) -> ValueId {
        let value = self.inner.global.value();
        self.inner
            .assignments
            .borrow_mut()
            .insert(value, Inst::Add(a, b));
        value
    }

    /// Perform a conditional jump to the given block with the specified inputs
    /// if the given condition is true.
    pub fn jump_if(
        &mut self,
        cond: ValueId,
        block: &Block,
        input: &[ValueId],
    ) -> Result<(), Error> {
        let expected = block.inner.entry.borrow().len();

        if expected != input.len() {
            return Err(Error::BlockInputMismatch {
                expected,
                actual: input.len(),
            });
        }

        self.inner
            .instructions
            .borrow_mut()
            .push(Inst::JumpIf(cond, block.id(), input.to_vec()));
        Ok(())
    }

    /// Unconditionally return from this the procedure this block belongs to.
    pub fn return_(&mut self, value: ValueId) {
        self.inner
            .instructions
            .borrow_mut()
            .push(Inst::Return(value));
    }
}

struct BlockInner {
    /// The identifier of the block.
    id: BlockId,
    /// Global shared stack machine state.
    global: Global,
    /// Entry variables.
    entry: RefCell<Vec<ValueId>>,
    /// Instructions being built.
    assignments: RefCell<HashMap<ValueId, Inst>>,
    /// Instructions that do not produce a value.
    instructions: RefCell<Vec<Inst>>,
}

/// Global construction state of the state machine.
#[derive(Debug, Clone, Default)]
struct Global {
    inner: Rc<GlobalInner>,
}

impl Global {
    /// Allocate a global variable.
    fn value(&self) -> ValueId {
        let id = self.inner.value.get();
        self.inner.value.set(id + 1);
        ValueId(id)
    }

    /// Allocate a global block identifier.
    fn block(&self) -> BlockId {
        let id = self.inner.block.get();
        self.inner.block.set(id + 1);
        BlockId(id)
    }

    /// Allocate a constant.
    fn constant(&self, constant: Constant) -> ConstId {
        let mut constants = self.inner.constants.borrow_mut();

        match &constant {
            Constant::Unit => return ConstId(0),
            Constant::String(s) => {
                let mut string_rev = self.inner.constant_string_rev.borrow_mut();

                if let Some(const_id) = string_rev.get(s) {
                    return *const_id;
                }

                let const_id = ConstId(constants.len());
                string_rev.insert(s.clone(), const_id);
                constants.push(constant);
                return const_id;
            }
            Constant::Bytes(b) => {
                let mut bytes_rev = self.inner.constant_bytes_rev.borrow_mut();

                if let Some(const_id) = bytes_rev.get(b) {
                    return *const_id;
                }

                let const_id = ConstId(constants.len());
                bytes_rev.insert(b.clone(), const_id);
                constants.push(constant);
                return const_id;
            }
            _ => (),
        }

        let const_id = ConstId(constants.len());
        constants.push(constant);
        const_id
    }
}

/// Inner state of the global.
#[derive(Debug)]
struct GlobalInner {
    /// Variable allocator.
    value: Cell<usize>,
    /// Block allocator.
    block: Cell<usize>,
    /// The values of constants.
    constants: RefCell<Vec<Constant>>,
    /// Constant strings that have already been allocated.
    constant_string_rev: RefCell<HashMap<Box<str>, ConstId>>,
    /// Constant byte arrays that have already been allocated.
    constant_bytes_rev: RefCell<HashMap<Box<[u8]>, ConstId>>,
}

impl Default for GlobalInner {
    fn default() -> Self {
        Self {
            value: Default::default(),
            block: Default::default(),
            constants: RefCell::new(vec![Constant::Unit]),
            constant_string_rev: Default::default(),
            constant_bytes_rev: Default::default(),
        }
    }
}

/// The central state machine assembler.
pub struct StateMachine {
    global: Global,
}

impl StateMachine {
    /// Construct a new empty state machine.
    pub fn new() -> Self {
        Self {
            global: Global::default(),
        }
    }

    /// Construct a new block associated with the state machine.
    pub fn block(&self) -> Block {
        Block::new(self.global.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::{Constant, Error, StateMachine};

    #[test]
    fn test_basic_sm() -> Result<(), Error> {
        let mut sm = StateMachine::new();

        let mut then = sm.block();
        let mut block = sm.block();

        let else_value = block.constant(Constant::Integer(1));
        then.return_value(else_value);

        // Define one entry variable to the block.
        let a = block.entry();
        let b = block.constant(Constant::Integer(42));
        let c = block.add(a, b);

        let d = block.cmp_lt(a, b);
        block.jump_if(d, &then, &[])?;
        block.return_unit();
        Ok(())
    }
}
