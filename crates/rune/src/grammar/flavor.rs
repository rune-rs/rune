use core::ops::{Deref, DerefMut};

use syntree::{Index, Storage, TreeIndex, Width};

use crate::alloc::{self, Vec};

pub(crate) struct Flavor;

pub struct AllocVec<T> {
    inner: Vec<T>,
}

impl<T> Deref for AllocVec<T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for AllocVec<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T> Storage<T> for AllocVec<T> {
    const EMPTY: Self = Self { inner: Vec::new() };

    type Error = alloc::Error;

    #[inline]
    fn with_capacity(capacity: usize) -> alloc::Result<Self> {
        Ok(Self {
            inner: Vec::try_with_capacity(capacity)?,
        })
    }

    #[inline]
    fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    #[inline]
    fn push(&mut self, item: T) -> alloc::Result<()> {
        self.inner.try_push(item)
    }
}

impl syntree::Flavor for Flavor {
    type Error = alloc::Error;
    type Index = u32;
    type Length = <u32 as Index>::Length;
    type Width = usize;
    type Pointer = <usize as Width>::Pointer;
    type Storage<T> = AllocVec<T>;
    type Indexes = AllocVec<TreeIndex<Self>>;
}
