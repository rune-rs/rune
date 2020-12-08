use crate::global::Global;
use crate::internal::commas;
use crate::{Assign, BlockId, Constant, Error, Phi, Term, Value, Var};
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
                assignments: RefCell::new(BTreeMap::new()),
                incomplete: RefCell::new(Vec::new()),
                term: RefCell::new(Term::Panic),
                ancestors: RefCell::new(Vec::new()),
            }),
        }
    }

    /// Read variable as a value.
    fn read_value(&self, var: Var) -> Result<Value, Error> {
        let assign = self.read(var)?;
        Ok(Value::Assign(assign))
    }

    /// Read the given variable, looking it up recursively in ancestor blocks
    /// and memoizing as needed.
    pub fn read(&self, var: Var) -> Result<Assign, Error> {
        // Local assignment that is already present.
        if let Some(assignment) = self.inner.assignments.borrow().get(&var) {
            return Ok(match &assignment.value {
                Value::Assign(assign) => assign.clone(),
                _ => assignment.assign.clone(),
            });
        }

        let assign = Assign::new(self.inner.global.static_id(), self.inner.id, var);

        // Place a node that breaks recursive dependencies.
        self.inner.assignments.borrow_mut().insert(
            assign.var(),
            Assignment {
                assign: assign.clone(),
                value: Value::Phi(Phi::new()),
            },
        );

        if self.inner.open.get() {
            self.inner
                .incomplete
                .borrow_mut()
                .push((assign.clone(), assign.var()));
        } else {
            self.add_phi(assign.var(), assign.var())?;
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
            if let Some(assignment) = block.inner.assignments.borrow().get(&var) {
                deps.push(assignment.assign.clone());
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
    pub fn assign(&self, id: Var, v: Var) -> Result<(), Error> {
        if id != v {
            let value = self.read_value(v)?;
            self.inner.assign(id, value)?;
        }

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
    pub fn seal(&self) -> Result<(), Error> {
        let open = self.inner.open.take();

        if open {
            let incomplete = std::mem::take(&mut *self.inner.incomplete.borrow_mut());

            for (var, var_to_read) in incomplete {
                self.add_phi(var.var(), var_to_read)?;
            }
        }

        Ok(())
    }

    /// Populate phi operands.
    fn add_phi(&self, var: Var, var_to_read: Var) -> Result<(), Error> {
        let deps = self.read_dependencies(var_to_read)?;

        if deps.len() <= 1 {
            if let Some(assign) = deps.into_iter().next() {
                if let Some(assignment) = self.inner.assignments.borrow_mut().remove(&var) {
                    assignment.assign.replace(&assign);
                    return Ok(());
                }
            }
        } else {
            if let Some(assignment) = self.inner.assignments.borrow_mut().get_mut(&var) {
                if let Value::Phi(phi) = &mut assignment.value {
                    phi.extend(deps);
                    return Ok(());
                }
            }
        }

        Err(Error::MissingPhiNode(var))
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
        Self::mark_control(self, block)?;
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

        Self::mark_control(self, then_block)?;
        Self::mark_control(self, else_block)?;

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
        Ok(())
    }

    /// Return from this the procedure this block belongs to.
    pub fn return_(&self, var: Var) -> Result<(), Error> {
        let var = self.read(var)?;
        *self.inner.term.borrow_mut() = Term::Return { var };
        Ok(())
    }

    fn mark_control(this: &Self, other: &Self) -> Result<(), Error> {
        if !other.inner.open.get() {
            return Err(Error::SealedBlockJump(this.inner.id, other.inner.id));
        }

        other.inner.ancestors.borrow_mut().push(this.id());
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

        for assign in self.0.inner.assignments.borrow().values() {
            writeln!(f, "  {} <- {}", assign.assign, assign.value.dump())?;
        }

        writeln!(f, "  {}", self.0.inner.term.borrow().dump())?;
        Ok(())
    }
}

struct Assignment {
    assign: Assign,
    value: Value,
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
    assignments: RefCell<BTreeMap<Var, Assignment>>,
    /// Collection of locally incomplete phi nodes.
    incomplete: RefCell<Vec<(Assign, Var)>>,
    /// Instructions that do not produce a value.
    term: RefCell<Term>,
    /// Ancestor blocks.
    ancestors: RefCell<Vec<BlockId>>,
}

impl BlockInner {
    /// Reassign the given variable.
    fn assign(&self, from: Var, value: Value) -> Result<(), Error> {
        let mut assignments = self.assignments.borrow_mut();

        // reassign a local var with a conflicting name.
        let value = if let Some(assignment) = assignments.remove(&from) {
            match value {
                Value::Assign(other) => {
                    // force every other user to redirect to the re-assigned
                    // assignment.
                    assignment.assign.replace(&other);
                    return Ok(());
                }
                value => value,
            }
        } else {
            // assigning a new variable.
            match value {
                Value::Assign(assign) => {
                    // reassign a local var with a name matching the value's
                    // var.
                    if let Some(assignment) = assignments.remove(&assign.var()) {
                        assignment.assign.update_local(from);
                        assignments.insert(from, assignment);
                        return Ok(());
                    }

                    Value::Assign(assign)
                }
                value => value,
            }
        };

        assignments.insert(
            from,
            Assignment {
                assign: Assign::new(self.global.static_id(), self.id, from),
                value,
            },
        );

        Ok(())
    }
}
