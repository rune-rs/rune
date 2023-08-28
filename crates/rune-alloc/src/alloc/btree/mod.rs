mod append;
mod borrow;
mod fix;
pub mod map;
mod mem;
mod merge_iter;
mod navigate;
mod node;
mod remove;
mod search;
pub mod set;
mod set_val;
mod split;

use core::cmp::Ordering;

use crate::alloc::AllocError;

trait Recover<Q: ?Sized> {
    type Key;

    fn get<C: ?Sized, E>(
        &self,
        cx: &mut C,
        key: &Q,
        cmp: fn(&mut C, &Q, &Q) -> Result<Ordering, E>,
    ) -> Result<Option<&Self::Key>, E>;

    fn take<C: ?Sized, E>(
        &mut self,
        cx: &mut C,
        key: &Q,
        cmp: fn(&mut C, &Q, &Q) -> Result<Ordering, E>,
    ) -> Result<Option<Self::Key>, E>;

    fn try_replace<C: ?Sized, E>(
        &mut self,
        cx: &mut C,
        key: Self::Key,
        cmp: fn(&mut C, &Q, &Q) -> Result<Ordering, E>,
    ) -> Result<Result<Option<Self::Key>, AllocError>, E>;
}
