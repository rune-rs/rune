use crate::{Block, Constant, Error, Value};
use hashbrown::HashMap;
use std::cell::{Cell, Ref, RefCell, RefMut};
use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;

/// The identifier of a constant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConstId(usize);

impl fmt::Display for ConstId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "C{}", self.0)
    }
}

/// A variable that can be used as block entries or temporaries.
/// Instructions typically produce and use vars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Var(usize);

impl fmt::Display for Var {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The identifier for the static assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StaticId(usize);

impl fmt::Display for StaticId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}", self.0)
    }
}

#[derive(Debug, Clone, Copy)]
struct SharedAssign {
    id: StaticId,
    block: BlockId,
}

/// The descriptor of a single assignment.
///
/// This has a shared interior, because the exact value being assigned might be
/// re-assigned during construction. Like when an existing assignment is being
/// replaced.
#[derive(Debug, Clone)]
pub struct Assign {
    shared: Rc<Cell<SharedAssign>>,
}

impl Assign {
    /// Construct a new reference to a variable in a different block.
    #[inline]
    pub(crate) fn new(id: StaticId, block: BlockId) -> Self {
        Self {
            shared: Rc::new(Cell::new(SharedAssign { id, block })),
        }
    }

    /// Replace this assignment with another.
    pub(crate) fn replace(&self, other: &Self) {
        self.shared.set(other.shared.get());
    }

    /// Get the assigned id.
    pub(crate) fn id(&self) -> StaticId {
        self.shared.get().id
    }
}

impl fmt::Display for Assign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let shared = self.shared.get();
        write!(f, "{}", shared.id)
    }
}

/// Identifier to a block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockId(usize);

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "${}", self.0)
    }
}

/// Global construction state of the state machine.
#[derive(Clone, Default)]
pub(crate) struct Global {
    inner: Rc<GlobalInner>,
}

impl Global {
    /// Get the inner values.
    pub(crate) fn values(&self) -> Ref<'_, Values> {
        self.inner.values.borrow()
    }

    /// Get the inner values mutably.
    pub(crate) fn values_mut(&self) -> RefMut<'_, Values> {
        self.inner.values.borrow_mut()
    }

    /// Mark that the given block returns from the procedure.
    pub(crate) fn mark_return(&self, block_id: BlockId) {
        self.inner.returns.borrow_mut().push(block_id);
    }

    /// Allocate a global variable.
    pub(crate) fn var(&self) -> Var {
        let id = self.inner.value.get();
        self.inner.value.set(id + 1);
        Var(id)
    }

    /// Allocate a static variable.
    pub(crate) fn static_id(&self) -> StaticId {
        let id = self.inner.statics.get();
        self.inner.statics.set(id + 1);
        StaticId(id)
    }

    /// Get accessor to underlying blocks.
    #[inline]
    pub(crate) fn blocks(&self) -> Blocks<'_> {
        Blocks {
            blocks: self.inner.blocks.borrow(),
        }
    }

    /// Allocate a block.
    pub(crate) fn block(&self, name: Option<Box<str>>) -> Block {
        let id = BlockId(self.inner.blocks.borrow().len());
        let block = Block::new(id, self.clone(), name);
        self.inner.blocks.borrow_mut().push(block.clone());
        block
    }

    /// Allocate a constant.
    pub(crate) fn constant(&self, constant: Constant) -> ConstId {
        let mut constants = self.inner.constants.borrow_mut();

        return match constant {
            Constant::Unit => ConstId(0),
            c => {
                let mut rev = self.inner.constants_rev.borrow_mut();

                if let Some(const_id) = rev.get(&c) {
                    return *const_id;
                }

                let const_id = ConstId(constants.len());
                rev.insert(c.clone(), const_id);
                constants.push(c);
                const_id
            }
        };
    }

    /// Access the collection of available constants.
    pub(crate) fn constants(&self) -> Ref<'_, [Constant]> {
        Ref::map(self.inner.constants.borrow(), |c| c.as_slice())
    }
}

#[derive(Default)]
pub(crate) struct Values {
    values: BTreeMap<StaticId, Value>,
}

impl Values {
    /// Remove the given value.
    pub(crate) fn remove(&mut self, id: StaticId) -> Option<Value> {
        self.values.remove(&id)
    }

    /// Insert the given value.
    pub(crate) fn insert(&mut self, id: StaticId, value: Value) {
        self.values.insert(id, value);
    }

    /// Get the value associated with the value.
    pub(crate) fn get(&self, id: StaticId) -> Option<&Value> {
        self.values.get(&id)
    }

    /// Get the value associated with the value.
    pub(crate) fn get_mut(&mut self, id: StaticId) -> Option<&mut Value> {
        self.values.get_mut(&id)
    }
}

/// Inner state of the global.
struct GlobalInner {
    /// Variable allocator.
    value: Cell<usize>,
    /// Static assignment id allocator.
    statics: Cell<usize>,
    /// Block allocator.
    blocks: RefCell<Vec<Block>>,
    /// The values of constants.
    constants: RefCell<Vec<Constant>>,
    /// Constant strings that have already been allocated.
    constants_rev: RefCell<HashMap<Constant, ConstId>>,
    /// The ID of blocks that return.
    returns: RefCell<Vec<BlockId>>,
    /// Values assocaited with the block.
    pub(crate) values: RefCell<Values>,
}

impl Default for GlobalInner {
    fn default() -> Self {
        Self {
            value: Default::default(),
            statics: Default::default(),
            blocks: Default::default(),
            constants: RefCell::new(vec![Constant::Unit]),
            constants_rev: Default::default(),
            returns: RefCell::new(Vec::new()),
            values: RefCell::new(Values::default()),
        }
    }
}

pub(crate) struct Blocks<'a> {
    blocks: Ref<'a, Vec<Block>>,
}

impl Blocks<'_> {
    /// Get the block corresponding to the given id.
    pub(crate) fn get(&self, id: BlockId) -> Result<&Block, Error> {
        match self.blocks.get(id.0) {
            Some(block) => Ok(block),
            None => Err(Error::MissingBlock(id)),
        }
    }
}
