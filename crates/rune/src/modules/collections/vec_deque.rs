use core::fmt::{self, Write};

use crate::no_std::collections;
use crate::no_std::prelude::*;

use crate::runtime::{Iterator, Protocol, Value, VmErrorKind, VmResult};
use crate::{Any, ContextError, Module};

pub(super) fn setup(module: &mut Module) -> Result<(), ContextError> {
    module.ty::<VecDeque>()?;
    module.function(["VecDeque", "new"], VecDeque::new)?;
    module.function(["VecDeque", "with_capacity"], VecDeque::with_capacity)?;
    module.function(["VecDeque", "from"], vecdeque_from)?;

    module.inst_fn("extend", VecDeque::extend)?;
    module.inst_fn("insert", VecDeque::insert)?;
    module.inst_fn("iter", VecDeque::iter)?;
    module.inst_fn("len", VecDeque::len)?;
    module.inst_fn("pop_back", VecDeque::pop_back)?;
    module.inst_fn("pop_front", VecDeque::pop_front)?;
    module.inst_fn("push_back", VecDeque::push_back)?;
    module.inst_fn("push_front", VecDeque::push_front)?;
    module.inst_fn("remove", VecDeque::remove)?;
    module.inst_fn("reserve", VecDeque::reserve)?;
    module.inst_fn("rotate_left", VecDeque::rotate_left)?;
    module.inst_fn("rotate_right", VecDeque::rotate_right)?;
    module.inst_fn(Protocol::INDEX_GET, VecDeque::get)?;
    module.inst_fn(Protocol::INDEX_SET, VecDeque::set)?;
    module.inst_fn(Protocol::INTO_ITER, VecDeque::iter)?;
    module.inst_fn(Protocol::STRING_DEBUG, VecDeque::string_debug)?;
    Ok(())
}

#[derive(Any, Clone, Default)]
#[rune(module = crate, item = ::std::collections)]
struct VecDeque {
    inner: collections::VecDeque<Value>,
}

impl VecDeque {
    fn new() -> VecDeque {
        Default::default()
    }

    fn with_capacity(count: usize) -> VecDeque {
        Self {
            inner: collections::VecDeque::with_capacity(count),
        }
    }

    /// Extend this VecDeque with something that implements the into_iter
    /// protocol.
    pub fn extend(&mut self, value: Value) -> VmResult<()> {
        let mut it = vm_try!(value.into_iter());

        while let Some(value) = vm_try!(it.next()) {
            self.push_back(value);
        }

        VmResult::Ok(())
    }

    fn rotate_left(&mut self, mid: usize) {
        self.inner.rotate_left(mid);
    }

    fn rotate_right(&mut self, mid: usize) {
        self.inner.rotate_left(mid);
    }

    fn push_front(&mut self, v: Value) {
        self.inner.push_front(v);
    }

    fn push_back(&mut self, v: Value) {
        self.inner.push_back(v);
    }

    fn pop_front(&mut self) -> Option<Value> {
        self.inner.pop_front()
    }

    fn pop_back(&mut self) -> Option<Value> {
        self.inner.pop_back()
    }

    fn remove(&mut self, index: usize) {
        self.inner.remove(index);
    }

    fn reserve(&mut self, index: usize) {
        self.inner.reserve(index);
    }

    fn len(&mut self) -> usize {
        self.inner.len()
    }

    fn get(&self, index: usize) -> VmResult<Value> {
        if index > self.inner.len() {
            return VmResult::err(VmErrorKind::OutOfRange {
                index: index.into(),
                length: self.inner.len().into(),
            });
        }

        VmResult::Ok(self.inner[index].clone())
    }

    fn set(&mut self, index: usize, value: Value) -> VmResult<()> {
        if index > self.inner.len() {
            return VmResult::err(VmErrorKind::OutOfRange {
                index: index.into(),
                length: self.inner.len().into(),
            });
        }

        self.inner[index] = value;
        VmResult::Ok(())
    }

    fn insert(&mut self, index: usize, value: Value) {
        self.inner.insert(index, value);
    }

    #[inline]
    fn iter(&self) -> Iterator {
        let iter = self.inner.clone().into_iter();
        Iterator::from("std::collections::VecDeque::Iter", iter)
    }

    #[inline]
    fn string_debug(&self, s: &mut String) -> fmt::Result {
        write!(s, "{:?}", self.inner)
    }
}

fn vecdeque_from(value: Value) -> VmResult<VecDeque> {
    let mut cont = VecDeque::new();
    let mut it = vm_try!(value.into_iter());

    while let Some(value) = vm_try!(it.next()) {
        cont.push_back(value);
    }

    VmResult::Ok(cont)
}
