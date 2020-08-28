//! The `std::iter` package.
//!
//! Note: This is a very simple prototype implementation.
//!
//! Contains functions such as:
//! * `range` to iterate over a range of integers.

use crate::{ContextError, Module};

#[derive(Debug)]
struct Rev {
    current: i64,
    start: i64,
}

impl Rev {
    fn next(&mut self) -> Option<i64> {
        if self.current <= self.start {
            return None;
        }

        self.current -= 1;
        Some(self.current)
    }
}

#[derive(Debug)]
struct Range {
    current: i64,
    end: i64,
}

impl Range {
    fn new(start: i64, end: i64) -> Self {
        Self {
            current: start,
            end,
        }
    }

    fn next(&mut self) -> Option<i64> {
        let value = self.current;

        if self.current < self.end {
            self.current += 1;
            return Some(value);
        }

        None
    }

    fn rev(self) -> Rev {
        Rev {
            current: self.end,
            start: self.current,
        }
    }
}

decl_external!(Range);
decl_external!(Rev);

/// Install the core package into the given functions namespace.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "iter"]);
    module.ty(&["Range"]).build::<Range>()?;
    module.ty(&["Rev"]).build::<Rev>()?;
    module.function(&["range"], Range::new)?;
    module.inst_fn(crate::NEXT, Range::next)?;
    module.inst_fn("rev", Range::rev)?;
    module.inst_fn("next", Rev::next)?;
    Ok(module)
}
