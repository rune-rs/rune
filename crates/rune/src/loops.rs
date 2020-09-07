use crate::ast;
use crate::compiler::Needs;
use crate::{
    error::{CompileError, CompileResult},
    Storage,
};
use runestick::{Label, Source};
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) struct LoopGuard {
    loops: Rc<RefCell<Vec<Loop>>>,
}

impl Drop for LoopGuard {
    fn drop(&mut self) {
        let empty = self.loops.borrow_mut().pop().is_some();
        debug_assert!(empty);
    }
}

/// Loops we are inside.
#[derive(Clone, Copy)]
pub(crate) struct Loop {
    /// The optional label of the loop.
    pub(crate) label: Option<ast::Label>,
    /// The end label of the loop.
    pub(crate) break_label: Label,
    /// The number of variables observed at the start of the loop.
    pub(crate) total_var_count: usize,
    /// If the loop needs a value.
    pub(crate) needs: Needs,
    /// Locals to drop when breaking.
    pub(crate) drop: Option<usize>,
}

pub(crate) struct Loops {
    loops: Rc<RefCell<Vec<Loop>>>,
}

impl Loops {
    /// Construct a new collection of loops.
    pub(crate) fn new() -> Self {
        Self {
            loops: Rc::new(RefCell::new(vec![])),
        }
    }

    /// Get the last loop context.
    pub(crate) fn last(&self) -> Option<Loop> {
        self.loops.borrow().last().copied()
    }

    /// Push loop information.
    pub(crate) fn push(&mut self, l: Loop) -> LoopGuard {
        self.loops.borrow_mut().push(l);

        LoopGuard {
            loops: self.loops.clone(),
        }
    }

    /// Find the loop with the matching label.
    pub(crate) fn walk_until_label(
        &self,
        storage: &Storage,
        source: &Source,
        expected: ast::Label,
    ) -> CompileResult<(Loop, Vec<usize>)> {
        use crate::traits::Resolve as _;

        let span = expected.span();
        let expected = expected.resolve(storage, source)?;
        let mut to_drop = Vec::new();

        for l in self.loops.borrow().iter().rev() {
            to_drop.extend(l.drop);

            let label = match l.label {
                Some(label) => label,
                None => {
                    continue;
                }
            };

            let label = label.resolve(storage, source)?;

            if expected == label {
                return Ok((*l, to_drop));
            }
        }

        Err(CompileError::MissingLabel { span })
    }

    /// Construct an iterator over all available scopes.
    pub(crate) fn iter(&self) -> impl Iterator<Item = Loop> {
        let loops = self.loops.borrow().clone();
        loops.into_iter()
    }
}
