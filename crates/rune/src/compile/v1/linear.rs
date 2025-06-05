use core::array;
use core::fmt;
use core::ops::{Deref, DerefMut};
use core::slice;

use crate::alloc::{vec, Vec};
use crate::compile;
use crate::runtime::inst;

use super::Address;

enum Addresses<'a, 'hir> {
    Single(Address<'a, 'hir>),
    List(Vec<Address<'a, 'hir>>),
}

#[must_use = "This should be freed with a call to Scopes::free_linear"]
pub(super) struct Linear<'a, 'hir> {
    addresses: Addresses<'a, 'hir>,
}

impl<'a, 'hir> Linear<'a, 'hir> {
    /// Construct a new linear address space.
    pub(super) fn new(list: Vec<Address<'a, 'hir>>) -> Self {
        Self {
            addresses: Addresses::List(list),
        }
    }

    /// Construct an empty linear allocation.
    pub(super) const fn empty() -> Self {
        Self {
            addresses: Addresses::List(Vec::new()),
        }
    }

    /// Represent a single address.
    pub(super) fn single(address: Address<'a, 'hir>) -> Self {
        Self {
            addresses: Addresses::Single(address),
        }
    }

    #[inline]
    pub(super) fn addr(&self) -> inst::Address {
        match &self.addresses {
            Addresses::Single(address) => address.addr(),
            Addresses::List(list) => list.first().map_or(inst::Address::INVALID, Address::addr),
        }
    }

    #[inline]
    pub(super) fn iter(&self) -> slice::Iter<'_, Address<'a, 'hir>> {
        <[_]>::iter(self)
    }

    #[inline]
    pub(super) fn iter_mut(&mut self) -> slice::IterMut<'_, Address<'a, 'hir>> {
        <[_]>::iter_mut(self)
    }

    #[inline]
    pub(super) fn free(self) -> compile::Result<()> {
        for addr in self.into_iter().rev() {
            addr.free()?;
        }

        Ok(())
    }

    #[inline]
    pub(super) fn free_non_dangling(self) -> compile::Result<()> {
        for addr in self.into_iter().rev() {
            addr.free_non_dangling()?;
        }

        Ok(())
    }

    #[inline]
    pub(super) fn forget(self) -> compile::Result<()> {
        for var in self {
            var.forget()?;
        }

        Ok(())
    }
}

impl<'b, 'a, 'hir> IntoIterator for &'b Linear<'a, 'hir> {
    type Item = &'b Address<'a, 'hir>;
    type IntoIter = slice::Iter<'b, Address<'a, 'hir>>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'b, 'a, 'hir> IntoIterator for &'b mut Linear<'a, 'hir> {
    type Item = &'b mut Address<'a, 'hir>;
    type IntoIter = slice::IterMut<'b, Address<'a, 'hir>>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<'a, 'hir> Deref for Linear<'a, 'hir> {
    type Target = [Address<'a, 'hir>];

    #[inline]
    fn deref(&self) -> &Self::Target {
        match &self.addresses {
            Addresses::Single(address) => slice::from_ref(address),
            Addresses::List(list) => list,
        }
    }
}

impl DerefMut for Linear<'_, '_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        match &mut self.addresses {
            Addresses::Single(address) => slice::from_mut(address),
            Addresses::List(list) => list,
        }
    }
}

impl fmt::Debug for Linear<'_, '_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<'a, 'hir> IntoIterator for Linear<'a, 'hir> {
    type Item = Address<'a, 'hir>;
    type IntoIter = IntoIter<'a, 'hir>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            inner: match self.addresses {
                Addresses::Single(address) => IntoIterRepr::Single([address].into_iter()),
                Addresses::List(list) => IntoIterRepr::Vec(list.into_iter()),
            },
        }
    }
}

enum IntoIterRepr<'a, 'hir> {
    Vec(vec::IntoIter<Address<'a, 'hir>>),
    Single(array::IntoIter<Address<'a, 'hir>, 1>),
}

pub(super) struct IntoIter<'a, 'hir> {
    inner: IntoIterRepr<'a, 'hir>,
}

impl<'a, 'hir> Iterator for IntoIter<'a, 'hir> {
    type Item = Address<'a, 'hir>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            IntoIterRepr::Vec(iter) => iter.next(),
            IntoIterRepr::Single(iter) => iter.next(),
        }
    }
}

impl ExactSizeIterator for IntoIter<'_, '_> {
    #[inline]
    fn len(&self) -> usize {
        match &self.inner {
            IntoIterRepr::Vec(iter) => iter.len(),
            IntoIterRepr::Single(iter) => iter.len(),
        }
    }
}

impl DoubleEndedIterator for IntoIter<'_, '_> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            IntoIterRepr::Vec(iter) => iter.next_back(),
            IntoIterRepr::Single(iter) => iter.next_back(),
        }
    }
}
