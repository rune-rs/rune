//! Builder for SSA.
//!
//! Based on the algorithm described in:
//!
//! Braun, Matthias, et al. "Simple and efficient construction of static single
//! assignment form." Compiler Construction: 22nd International Conference, CC
//! 2013, Held as Part of the European Joint Conferences on Theory and Practice
//! of Software, ETAPS 2013, Rome, Italy, March 16-24, 2013. Proceedings 22.
//! Springer Berlin Heidelberg, 2013.
//!
//! See: https://pp.info.uni-karlsruhe.de/uploads/publikationen/braun13cc.pdf

#[cfg(test)]
mod tests;

use core::fmt;
use core::marker::PhantomData;
use core::mem::{replace, take};
use core::ops::{Index, IndexMut};

use rune_macros::InstDisplay;

use crate::no_std::collections::{HashMap, HashSet};
use crate::no_std::prelude::*;

trait Id {
    fn new(index: usize) -> Self;
    fn get(self) -> usize;
}

struct IndexedVec<I, T> {
    data: Vec<T>,
    _index: PhantomData<I>,
}

impl<I, T> Default for IndexedVec<I, T> {
    fn default() -> Self {
        Self {
            data: Default::default(),
            _index: PhantomData,
        }
    }
}

impl<I, T> IndexedVec<I, T>
where
    I: Id,
{
    #[inline]
    fn push(&mut self, value: T) -> I {
        let index = I::new(self.data.len());
        self.data.push(value);
        index
    }

    /// Get the next index that will be inserted.
    #[inline]
    fn next(&self) -> I {
        I::new(self.data.len())
    }

    /// Iterate over data.
    #[inline]
    fn iter(&self) -> impl Iterator<Item = (I, &T)> {
        self.data
            .iter()
            .enumerate()
            .map(|(i, data)| (I::new(i), data))
    }
}

impl<I, T> Index<I> for IndexedVec<I, T>
where
    I: Id,
{
    type Output = T;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        &self.data[index.get()]
    }
}

impl<I, T> IndexMut<I> for IndexedVec<I, T>
where
    I: Id,
{
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.data[index.get()]
    }
}

#[derive(Default)]
pub(crate) struct Ssa {
    blocks: IndexedVec<Block, SsaBlock>,
    values: IndexedVec<Value, SsaValue>,
    variables: usize,
}

impl Ssa {
    /// Iterate over instructions.
    pub(crate) fn instructions(&self, block: Block) -> impl Iterator<Item = (Value, &SsaValue)> {
        self.blocks[block]
            .values
            .iter()
            .map(|&v| (v, &self.values[v]))
    }

    /// Define a new block.
    pub(crate) fn block(&mut self) -> Block {
        self.blocks.push(SsaBlock::default())
    }

    /// Define a new variable.
    pub(crate) fn variable(&mut self) -> Variable {
        let variable = Variable(self.variables);
        self.variables = self.variables.wrapping_add(1);
        variable
    }

    /// Write a variable.
    pub(crate) fn write(&mut self, block: Block, variable: Variable, kind: Inst) {
        let value = self.values.next();

        kind.visit_operands(|v| {
            self.values[*v].users.insert(value);
        });

        self.values.push(SsaValue::new(block, kind));
        self.blocks[block].defs.insert(variable, value);
        self.blocks[block].values.push(value);
    }

    /// Read a variable.
    pub(crate) fn read(&mut self, block: Block, variable: Variable) -> Value {
        // Variable is defined in the current block.
        let Some(cur) = self.blocks[block].defs.get_mut(&variable) else {
            let value = self.read_recursive(block, variable);
            self.blocks[block].defs.insert(variable, value);
            self.blocks[block].values.push(value);
            return value;
        };

        // If a value has been deleted, recursively lookup the value it's been
        // deleted for. Update its memoization for future lookups.
        loop {
            let Some(v) = self.values[*cur].replaced else {
                return *cur;
            };

            *cur = v;
        }
    }

    /// Read a variable recursively
    fn read_recursive(&mut self, block: Block, variable: Variable) -> Value {
        if !self.blocks[block].sealed {
            let value = self.values.push(SsaValue::new(
                block,
                Inst::Phi {
                    operands: Vec::new(),
                },
            ));
            self.blocks[block].incomplete_phis.push((variable, value));
            return value;
        };

        // Trivial case, where we only have one predecessor.
        if let &[block] = self.blocks[block].preds.as_slice() {
            return self.read(block, variable);
        }

        let value = self.values.push(SsaValue::new(
            block,
            Inst::Phi {
                operands: Vec::new(),
            },
        ));

        // Ensure that we short circuit recursion by inserting a placeholder phi
        // first.
        self.blocks[block].defs.insert(variable, value);
        self.add_phi_operands(variable, value)
    }

    /// Add phi operands.
    fn add_phi_operands(&mut self, variable: Variable, phi: Value) -> Value {
        let mut inst = replace(&mut self.values[phi].inst, Inst::Empty);

        let Inst::Phi { operands } = &mut inst else {
            self.values[phi].inst = inst;
            return phi;
        };

        let block = self.values[phi].block;
        let preds = take(&mut self.blocks[block].preds);

        for &block in &preds {
            let value = self.read(block, variable);
            operands.push(value);
            self.values[value].users.insert(phi);
        }

        self.values[phi].inst = inst;
        self.blocks[block].preds = preds;
        self.try_remove_trivial_phi(phi)
    }

    fn try_remove_trivial_phi(&mut self, phi: Value) -> Value {
        let inst = replace(&mut self.values[phi].inst, Inst::Empty);

        let Inst::Phi { operands } = &inst else {
            self.values[phi].inst = inst;
            return phi;
        };

        let mut same = None;

        for &op in operands {
            // Unique value or self−reference
            if matches!(same, Some(same) if same == op) || op == phi {
                continue;
            }

            if same.is_some() {
                self.values[phi].inst = inst;
                // The phi merges at least two values: not trivial
                return phi;
            }

            same = Some(op);
        }

        let ssa_value_users = take(&mut self.values[phi].users);

        let replacement = if let Some(same) = same {
            same
        } else {
            self.values
                .push(SsaValue::new(self.values[phi].block, Inst::Empty))
        };

        for &op in operands {
            self.values[op].users.remove(&phi);
            self.values[op].users.insert(replacement);
        }

        for &u in &ssa_value_users {
            self.values[u].inst.visit_operands_mut(|v| {
                if *v == phi {
                    *v = replacement
                }
            });

            self.try_remove_trivial_phi(u);
        }

        self.values[phi].inst = inst;
        self.values[phi].users = ssa_value_users;
        self.values[phi].replaced = Some(replacement);
        replacement
    }

    pub(crate) fn seal_block(&mut self, block: Block) {
        // Already sealed.
        if self.blocks[block].sealed {
            return;
        }

        for (variable, phi) in take(&mut self.blocks[block].incomplete_phis) {
            self.add_phi_operands(variable, phi);
        }

        self.blocks[block].sealed = true;
    }

    /// Add a predecessor block.
    fn add_pred(&mut self, a: Block, pred: Block) {
        self.blocks[a].preds.push(pred);
    }
}

macro_rules! index {
    ($name:ident, $fmt:literal) => {
        #[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
        #[repr(transparent)]
        pub(crate) struct $name(usize);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, $fmt, self.0)
            }
        }

        impl Id for $name {
            fn new(index: usize) -> Self {
                Self(index)
            }

            fn get(self) -> usize {
                self.0
            }
        }
    };
}

#[derive(Default)]
struct SsaBlock {
    preds: Vec<Block>,
    values: Vec<Value>,
    incomplete_phis: Vec<(Variable, Value)>,
    sealed: bool,
    defs: HashMap<Variable, Value>,
}

index!(Block, "b{0}");
index!(Variable, "_{0}");

pub(crate) struct SsaValue {
    block: Block,
    inst: Inst,
    users: HashSet<Value>,
    replaced: Option<Value>,
}

impl SsaValue {
    fn new(block: Block, inst: Inst) -> Self {
        Self {
            block,
            inst,
            users: HashSet::new(),
            replaced: None,
        }
    }
}

impl fmt::Display for SsaValue {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(deleted) = self.replaced {
            write!(f, "{} (replaced: {})", self.inst, deleted)
        } else {
            write!(f, "{}", self.inst)
        }
    }
}

index!(Value, "v{0}");

#[derive(InstDisplay)]
pub(crate) enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

/// The kind of an instruction.
#[derive(InstDisplay)]
pub(crate) enum Inst {
    /// An empty variable.
    Empty,
    /// A phi block.
    Phi {
        #[inst_display(display_with = display_array)]
        operands: Vec<Value>,
    },
    /// Perform addition.
    Binary { op: BinOp, lhs: Value, rhs: Value },
    /// Perform an in-place addition.
    Assign { op: BinOp, input: Value, rhs: Value },
}

impl Inst {
    fn visit_operands_mut<V>(&mut self, mut v: V)
    where
        V: FnMut(&mut Value),
    {
        match self {
            Inst::Empty => {}
            Inst::Phi { operands } => {
                operands.iter_mut().for_each(v);
            }
            Inst::Binary {
                op: BinOp,
                lhs,
                rhs,
            } => {
                v(lhs);
                v(rhs);
            }
            Inst::Assign {
                op: BinOp,
                input,
                rhs,
            } => {
                v(input);
                v(rhs);
            }
        }
    }

    fn visit_operands<V>(&self, mut v: V)
    where
        V: FnMut(&Value),
    {
        match self {
            Inst::Empty => {}
            Inst::Phi { operands } => {
                operands.iter().for_each(v);
            }
            Inst::Binary { lhs, rhs, .. } => {
                v(lhs);
                v(rhs);
            }
            Inst::Assign { input, rhs, .. } => {
                v(input);
                v(rhs);
            }
        }
    }
}

fn display_array<T>(array: &[T]) -> impl fmt::Display + '_
where
    T: fmt::Display,
{
    struct Display<'a, T>(&'a [T]);

    impl<T> fmt::Display for Display<'_, T>
    where
        T: fmt::Display,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let mut it = self.0.iter();

            write!(f, "[")?;
            let last = it.next_back();

            for value in it {
                write!(f, "{value}, ")?;
            }

            if let Some(last) = last {
                last.fmt(f)?;
            }

            write!(f, "]")?;
            Ok(())
        }
    }

    Display(array)
}
