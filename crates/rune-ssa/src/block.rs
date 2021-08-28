use crate::global::Global;
use crate::internal::commas;
use crate::{Assign, BlockId, Constant, Error, Phi, StaticId, Term, Value, Var};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::rc::Rc;

/// Macro to help build a unary op.
macro_rules! block_unary_op {
    ($new:ident, $assign:ident, $variant:ident, $doc:literal) => {
        #[doc = $doc]
        pub fn $new(&self, a: Var) -> Result<Var, Error> {
            let a = self.read(a)?;
            self.assign_new(Value::$variant(a))
        }

        #[doc = $doc]
        pub fn $assign(&self, id: Var, a: Var) -> Result<(), Error> {
            let a = self.read(a)?;
            self.assign_value(id, Value::$variant(a))?;
            Ok(())
        }
    };
}

/// Macro to help build a binary op.
macro_rules! block_binary_op {
    ($new:ident, $assign:ident, $variant:ident, $doc:literal) => {
        #[doc = $doc]
        pub fn $new(&self, a: Var, b: Var) -> Result<Var, Error> {
            let a = self.read(a)?;
            let b = self.read(b)?;
            self.assign_new(Value::$variant(a, b))
        }

        #[doc = $doc]
        pub fn $assign(&self, id: Var, a: Var, b: Var) -> Result<(), Error> {
            let a = self.read(a)?;
            let b = self.read(b)?;
            self.assign_value(id, Value::$variant(a, b))?;
            Ok(())
        }
    };
}

/// A block containing a sequence of assignments.
///
/// A block carries a definition of its entry.
/// The entry is the sequence of input variables the block expects.
#[derive(Clone)]
pub struct Block {
    inner: Rc<BlockInner>,
}

impl Block {
    /// Construct a new empty block.
    pub(crate) fn new(id: BlockId, global: Global, name: Option<Box<str>>) -> Self {
        Self {
            inner: Rc::new(BlockInner {
                id,
                inputs: Cell::new(0),
                name,
                global,
                open: Cell::new(true),
                vars: RefCell::new(BTreeMap::new()),
                incomplete: RefCell::new(Vec::new()),
                term: RefCell::new(Term::Panic),
                ancestors: RefCell::new(Vec::new()),
            }),
        }
    }

    /// The name of the block.
    pub fn name(&self) -> Option<&str> {
        self.inner.name.as_deref()
    }

    /// Read the given variable, looking it up recursively in ancestor blocks
    /// and memoizing as needed.
    pub fn read(&self, var: Var) -> Result<Assign, Error> {
        // Local assignment that is already present.
        if let Some(assign) = self.inner.vars.borrow().get(&var) {
            return Ok(assign.clone());
        }

        let id = self.inner.global.static_id();
        let assign = Assign::new(id, self.id());

        self.inner
            .global
            .values_mut()
            .insert(id, Value::Phi(Phi::new()));

        self.inner.vars.borrow_mut().insert(var, assign.clone());

        if self.inner.open.get() {
            self.inner
                .incomplete
                .borrow_mut()
                .push((assign.clone(), var));
        } else {
            self.add_phi(id, var)?;
        }

        Ok(assign)
    }

    /// Read the given dependencies recursively.
    fn read_dependencies(&self, var: Var) -> Result<Vec<Assign>, Error> {
        let blocks = self.inner.global.blocks();
        let mut deps = Vec::new();

        let mut queue = VecDeque::new();

        for ancestor in &*self.inner.ancestors.borrow() {
            queue.push_back(blocks.get(*ancestor)?);
        }

        while let Some(block) = queue.pop_front() {
            if let Some(assign) = block.inner.vars.borrow().get(&var) {
                deps.push(assign.clone());
                continue;
            }

            for ancestor in &*block.inner.ancestors.borrow() {
                queue.push_back(blocks.get(*ancestor)?);
            }
        }

        Ok(deps)
    }

    /// Assign an instruction to a new vvar.
    fn assign_new(&self, value: Value) -> Result<Var, Error> {
        let var = self.inner.global.var();
        self.assign_value(var, value)?;
        Ok(var)
    }

    /// Assign an instruction to an existing var.
    fn assign_value(&self, id: Var, value: Value) -> Result<(), Error> {
        self.inner.assign(id, value)?;
        Ok(())
    }

    /// Assign a variable.
    pub fn assign(&self, var: Var, to_read: Var) -> Result<(), Error> {
        let assign = self.read(to_read)?;
        self.inner.vars.borrow_mut().insert(var, assign);
        Ok(())
    }

    /// Define an input into the block.
    pub fn input(&self) -> Result<Var, Error> {
        let id = self.inner.global.var();
        let input = self.inner.inputs.get();
        self.inner.inputs.set(input + 1);
        self.assign_value(id, Value::Input(input))?;
        Ok(id)
    }

    /// Finalize the block.
    pub(crate) fn seal(&self) -> Result<(), Error> {
        let open = self.inner.open.take();

        if open {
            let incomplete = std::mem::take(&mut *self.inner.incomplete.borrow_mut());

            for (assign, var_to_read) in incomplete {
                self.add_phi(assign.id(), var_to_read)?;
            }
        }

        Ok(())
    }

    /// Populate phi operands.
    fn add_phi(&self, id: StaticId, var_to_read: Var) -> Result<(), Error> {
        let deps = self.read_dependencies(var_to_read)?;

        if deps.len() <= 1 {
            if let Some(assign) = deps.into_iter().next() {
                if let Some(Value::Phi(..)) = self.inner.global.values_mut().remove(id) {
                    let old = self
                        .inner
                        .vars
                        .borrow_mut()
                        .insert(var_to_read, assign.clone());

                    if let Some(existing) = old {
                        existing.replace(&assign);
                    }

                    return Ok(());
                }
            }
        } else if let Some(Value::Phi(phi)) = self.inner.global.values_mut().get_mut(id) {
            phi.extend(deps);
            return Ok(());
        }

        Err(Error::MissingPhiNode(id))
    }

    /// Get the identifier of the block.
    #[inline]
    pub fn id(&self) -> BlockId {
        self.inner.id
    }

    /// Perform a diagnostical dump of a block.
    #[inline]
    pub fn dump(&self) -> BlockDump<'_> {
        BlockDump(self)
    }

    /// Define a unit.
    pub fn unit(&self) -> Result<Var, Error> {
        self.constant(Constant::Unit)
    }

    /// Assign a unit.
    pub fn assign_unit(&self, id: Var) -> Result<(), Error> {
        self.assign_constant(id, Constant::Unit)?;
        Ok(())
    }

    /// Define a constant.
    pub fn constant(&self, constant: Constant) -> Result<Var, Error> {
        let const_id = self.inner.global.constant(constant);
        self.assign_new(Value::Const(const_id))
    }

    /// Assign a constant.
    pub fn assign_constant(&self, id: Var, constant: Constant) -> Result<(), Error> {
        let const_id = self.inner.global.constant(constant);
        self.assign_value(id, Value::Const(const_id))?;
        Ok(())
    }

    block_unary_op!(not, assign_not, Not, "Compute `!arg`.");
    block_binary_op!(add, assign_add, Add, "Compute `lhs + rhs`.");
    block_binary_op!(sub, assign_sub, Sub, "Compute `lhs - rhs`.");
    block_binary_op!(div, assign_div, Div, "Compute `lhs / rhs`.");
    block_binary_op!(mul, assign_mul, Mul, "Compute `lhs * rhs`.");
    block_binary_op!(cmp_lt, assign_cmp_lt, CmpLt, "Compare if `lhs < rhs`.");
    block_binary_op!(cmp_lte, assign_cmp_lte, CmpLte, "Compare if `lhs <= rhs`.");
    block_binary_op!(cmp_eq, assign_cmp_eq, CmpEq, "Compare if `lhs == rhs`.");
    block_binary_op!(cmp_gt, assign_cmp_gt, CmpGt, "Compare if `lhs > rhs`.");
    block_binary_op!(cmp_gte, assign_cmp_gte, CmpGte, "Compare if `lhs >= rhs`.");

    /// Perform an unconditional jump to the given block with the specified
    /// inputs.
    pub fn jump(&self, block: &Block) -> Result<(), Error> {
        self.mark_control(block)?;

        *self.inner.term.borrow_mut() = Term::Jump { block: block.id() };
        Ok(())
    }

    /// Perform a conditional jump to the given block with the specified inputs
    /// if the given condition is true.
    pub fn jump_if(
        &self,
        condition: Var,
        then_block: &Block,
        else_block: &Block,
    ) -> Result<(), Error> {
        let condition = self.read(condition)?;

        self.mark_control(then_block)?;
        self.mark_control(else_block)?;

        *self.inner.term.borrow_mut() = Term::JumpIf {
            condition,
            then_block: then_block.id(),
            else_block: else_block.id(),
        };

        Ok(())
    }

    /// Return from this the procedure this block belongs to.
    pub fn return_unit(&self) -> Result<(), Error> {
        let var = self.unit()?;
        let var = self.read(var)?;

        *self.inner.term.borrow_mut() = Term::Return { var };
        self.inner.global.mark_return(self.id());
        Ok(())
    }

    /// Return from this the procedure this block belongs to.
    pub fn return_(&self, var: Var) -> Result<(), Error> {
        let var = self.read(var)?;

        *self.inner.term.borrow_mut() = Term::Return { var };
        self.inner.global.mark_return(self.id());
        Ok(())
    }

    fn mark_control(&self, other: &Self) -> Result<(), Error> {
        if !other.inner.open.get() {
            return Err(Error::SealedBlockJump(self.id(), other.inner.id));
        }

        other.inner.ancestors.borrow_mut().push(self.id());
        Ok(())
    }
}

pub struct BlockDump<'a>(&'a Block);

impl fmt::Display for BlockDump<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ancestors = self.0.inner.ancestors.borrow();

        write!(f, "{}", self.0.id())?;

        if self.0.inner.open.get() {
            write!(f, " (open)")?;
        }

        if ancestors.is_empty() {
            write!(f, ":")?;
        } else {
            write!(f, ": {}", commas(&ancestors[..]))?;
        }

        if let Some(name) = &self.0.inner.name {
            write!(f, " // {}", name)?;
        }

        writeln!(f)?;

        for (var, assign) in self.0.inner.vars.borrow().iter() {
            if let Some(value) = self.0.inner.global.values().get(assign.id()) {
                writeln!(f, "  {}: {} <- {}", var, assign, value.dump())?;
            } else {
                writeln!(f, "  {}: {} <- ?", var, assign)?;
            }
        }

        writeln!(f, "  {}", self.0.inner.term.borrow().dump())?;
        Ok(())
    }
}

struct BlockInner {
    /// The identifier of the block.
    id: BlockId,
    /// The number of inputs in the block.
    inputs: Cell<usize>,
    /// If the block is finalized or not.
    ///
    /// Control flows can only be added to non-finalized blocks.
    open: Cell<bool>,
    /// The (optional) name of the block for debugging and symbolic purposes.
    name: Option<Box<str>>,
    /// Global shared stack machine state.
    global: Global,
    /// Instructions being built.
    vars: RefCell<BTreeMap<Var, Assign>>,
    /// Collection of locally incomplete phi nodes.
    incomplete: RefCell<Vec<(Assign, Var)>>,
    /// Instructions that do not produce a value.
    term: RefCell<Term>,
    /// Ancestor blocks.
    ancestors: RefCell<Vec<BlockId>>,
}

impl BlockInner {
    /// Reassign the given variable.
    fn assign(&self, var: Var, value: Value) -> Result<(), Error> {
        let id = self.global.static_id();
        self.vars.borrow_mut().insert(var, Assign::new(id, self.id));
        self.global.values_mut().insert(id, value);
        Ok(())
    }
}
