//! A simple label used to jump to a code location.

use core::cell::Cell;
use core::fmt;
use core::num::NonZeroUsize;

use crate as rune;
use crate::alloc::borrow::Cow;
use crate::alloc::prelude::*;
use ::rust_alloc::rc::Rc;

use serde::{Deserialize, Serialize};

/// A label that can be jumped to.
#[derive(Debug, TryClone)]
pub(crate) struct Label {
    pub(crate) name: &'static str,
    pub(crate) index: usize,
    #[try_clone(with = Rc::clone)]
    jump: Rc<Cell<Option<NonZeroUsize>>>,
}

impl Label {
    /// Construct a new label.
    pub(crate) fn new(name: &'static str, index: usize) -> Self {
        Self {
            name,
            index,
            jump: Rc::new(Cell::new(None)),
        }
    }

    /// Get jump.
    pub(crate) fn jump(&self) -> Option<usize> {
        Some(self.jump.get()?.get().wrapping_sub(1))
    }

    /// Set jump.
    pub(crate) fn set_jump(&self, jump: usize) -> bool {
        let Some(jump) = NonZeroUsize::new(jump.wrapping_add(1)) else {
            return false;
        };

        self.jump.replace(Some(jump));
        true
    }

    /// Convert into owned label.
    pub(crate) fn to_debug_label(&self) -> DebugLabel {
        DebugLabel {
            name: self.name.into(),
            index: self.index,
            jump: self.jump.get(),
        }
    }
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(jump) = self.jump() {
            write!(f, "{}_{} ({jump})", self.name, self.index)
        } else {
            write!(f, "{}_{}", self.name, self.index)
        }
    }
}

/// A label that can be jumped to.
#[derive(Debug, TryClone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DebugLabel {
    /// The name of the label.
    name: Cow<'static, str>,
    /// The index of the label.
    index: usize,
    /// The jump index of the label.
    jump: Option<NonZeroUsize>,
}

impl DebugLabel {
    /// Get jump.
    pub(crate) fn jump(&self) -> Option<usize> {
        Some(self.jump?.get().wrapping_sub(1))
    }
}

impl fmt::Display for DebugLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}_{}", self.name, self.index)?;

        if let Some(jump) = self.jump() {
            write!(f, " ({jump})")?;
        }

        Ok(())
    }
}
