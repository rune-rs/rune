use crate::{Block, Constant, Error};
use hashbrown::HashMap;
use std::cell::{Cell, Ref, RefCell};
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

/// The descriptor of a single assignment.
///
/// This has a shared interior, because the exact value being assigned might be
/// re-assigned during construction. Like when an existing assignment is being
/// replaced.
#[derive(Debug, Clone)]
pub struct Assign {
    shared: Rc<Cell<(StaticId, BlockId, Var)>>,
}

impl Assign {
    /// Construct a new reference to a variable in a different block.
    #[inline]
    pub(crate) fn new(id: StaticId, block: BlockId, var: Var) -> Self {
        Self {
            shared: Rc::new(Cell::new((id, block, var))),
        }
    }

    /// Set the value of thie block var to another var.
    pub(crate) fn replace(&self, other: &Self) {
        let var = other.shared.get();
        self.shared.set(var);
    }

    /// Update the local variable this assignment is pointing towards.
    pub(crate) fn update_local(&self, var: Var) {
        let (id, block, _) = self.shared.get();
        self.shared.set((id, block, var));
    }

    /// Access the var this belongs to.
    pub(crate) fn var(&self) -> Var {
        self.shared.get().2
    }
}

impl fmt::Display for Assign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (id, _, _) = self.shared.get();
        write!(f, "{}", id)
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
}

impl Default for GlobalInner {
    fn default() -> Self {
        Self {
            value: Default::default(),
            statics: Default::default(),
            blocks: Default::default(),
            constants: RefCell::new(vec![Constant::Unit]),
            constants_rev: Default::default(),
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
