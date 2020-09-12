//! The `std::iter` module.

use crate::{ContextError, Module};

/// Construct the `std::iter` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(&["std", "iter"]);
    module.ty::<Range>()?;
    module.ty::<Rev>()?;
    module.function(&["range"], Range::new)?;
    module.inst_fn(crate::INTO_ITER, Range::into_iter)?;
    module.inst_fn(crate::NEXT, Range::next)?;
    module.inst_fn("rev", Range::rev)?;
    module.inst_fn(crate::INTO_ITER, Rev::into_iter)?;
    module.inst_fn(crate::NEXT, Rev::next)?;
    Ok(module)
}

#[derive(Debug)]
struct Rev {
    current: i64,
    start: i64,
}

impl Iterator for Rev {
    type Item = i64;

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

    fn rev(self) -> Rev {
        Rev {
            current: self.end,
            start: self.current,
        }
    }
}

impl Iterator for Range {
    type Item = i64;

    fn next(&mut self) -> Option<i64> {
        let value = self.current;

        if self.current < self.end {
            self.current += 1;
            return Some(value);
        }

        None
    }
}

crate::__internal_impl_any!(Range);
crate::__internal_impl_any!(Rev);
