// Note: A fair amount of code in this module is duplicated from the Rust
// project under the MIT license.
//
// https://github.com/rust-lang/rust
//
// Copyright 2014-2020 The Rust Project Developers

use core::cmp;
use core::fmt;
use core::iter;

use rust_alloc::boxed::Box;

use crate as rune;
use crate::alloc;
use crate::alloc::prelude::*;
use crate::runtime::{FromValue, Function, Panic, ToValue, Value, VmErrorKind, VmResult, Protocol};
use crate::Any;

/// An owning iterator.
#[derive(Any)]
#[rune(builtin, static_type = ITERATOR_TYPE)]
#[rune(from_value = Value::into_iterator, from_value_ref = Value::into_iterator_ref, from_value_mut = Value::into_iterator_mut)]
pub struct RuntimeIterator {
    iter: Value,
}

impl RuntimeIterator {
    #[inline]
    pub(crate) fn size_hint(&self) -> VmResult<(usize, Option<usize>)> {
        self.iter.call_protocol(Protocol::SIZE_HINT, ())
    }

    #[inline]
    pub(crate) fn next(&self) -> VmResult<Option<Value>> {
        self.iter.call_protocol(Protocol::NEXT, ())
    }

    #[inline]
    pub(crate) fn next_back(&self) -> VmResult<Option<Value>> {
        self.iter.call_protocol(Protocol::NEXT_BACK, ())
    }

    #[inline]
    pub(crate) fn find(&mut self, find: Function) -> VmResult<Option<Value>> {
        while let Some(value) = vm_try!(self.next()) {
            if vm_try!(find.call::<bool>((value.clone(),))) {
                return VmResult::Ok(Some(value));
            }
        }

        VmResult::Ok(None)
    }

    #[inline]
    pub(crate) fn all(&mut self, find: Function) -> VmResult<bool> {
        while let Some(value) = vm_try!(self.next()) {
            let result = vm_try!(find.call::<bool>((value.clone(),)));

            if !result {
                return VmResult::Ok(false);
            }
        }

        VmResult::Ok(true)
    }

    #[inline]
    pub(crate) fn any(&mut self, find: Function) -> VmResult<bool> {
        while let Some(value) = vm_try!(self.next()) {
            if vm_try!(find.call::<bool>((value.clone(),))) {
                return VmResult::Ok(true);
            }
        }

        VmResult::Ok(false)
    }

    #[inline]
    pub(crate) fn chain(self, other: Value) -> VmResult<Self> {
        let other = vm_try!(other.into_iter());

        VmResult::Ok(Self {
            iter: IterRepr::Chain(Box::new(Chain {
                a: Some(self.iter),
                b: Some(other.iter),
            })),
        })
    }

    #[inline]
    pub(crate) fn rev(self) -> VmResult<Self> {
        VmResult::Ok(Self {
            iter: match self.iter {
                // NB: reversing a reversed iterator restores the original
                // iterator.
                IterRepr::Rev(rev) => rev.iter,
                iter => IterRepr::Rev(Box::new(Rev { iter })),
            },
        })
    }

    #[inline]
    pub(crate) fn skip(self, n: usize) -> Self {
        Self {
            iter: IterRepr::Skip(Box::new(Skip { iter: self.iter, n })),
        }
    }

    #[inline]
    pub(crate) fn take(self, n: usize) -> Self {
        Self {
            iter: IterRepr::Take(Box::new(Take { iter: self.iter, n })),
        }
    }

    #[inline]
    pub(crate) fn count(&mut self) -> VmResult<usize> {
        let mut c = 0;

        while vm_try!(self.iter.call_protocol::<Option<Value>>(Protocol::NEXT, ())).is_some() {
            c += 1;
        }

        VmResult::Ok(c)
    }

    #[inline]
    pub(crate) fn peekable(self) -> Self {
        Self {
            iter: match self.iter {
                IterRepr::Peekable(peekable) => IterRepr::Peekable(peekable),
                iter => IterRepr::Peekable(Box::new(Peekable { iter, peeked: None })),
            },
        }
    }

    #[inline]
    pub(crate) fn peek(&mut self) -> VmResult<Option<Value>> {
        match &mut self.iter {
            IterRepr::Peekable(peekable) => peekable.peek(),
            _ => VmResult::err(Panic::custom(vm_try!(format_args!(
                "`{:?}` is not a peekable iterator",
                self.iter
            )
            .try_to_string()))),
        }
    }

    #[inline]
    pub(crate) fn collect<T>(mut self) -> VmResult<alloc::Vec<T>>
    where
        T: FromValue,
    {
        let (cap, _) = self.iter.size_hint();
        let mut vec = vm_try!(alloc::Vec::try_with_capacity(cap));

        while let Some(value) = vm_try!(self.next()) {
            vm_try!(vec.try_push(vm_try!(T::from_value(value))));
        }

        VmResult::Ok(vec)
    }

    #[inline]
    pub(crate) fn fold(mut self, mut accumulator: Value, f: Function) -> VmResult<Value> {
        while let Some(value) = vm_try!(self.next()) {
            accumulator = vm_try!(f.call((accumulator, value.clone())));
        }

        VmResult::Ok(accumulator)
    }

    #[inline]
    pub(crate) fn reduce(mut self, f: Function) -> VmResult<Option<Value>> {
        let Some(mut accumulator) = vm_try!(self.next()) else {
            return VmResult::Ok(None);
        };

        while let Some(value) = vm_try!(self.next()) {
            accumulator = vm_try!(f.call((accumulator, value.clone())));
        }

        VmResult::Ok(Some(accumulator))
    }

    #[inline]
    pub(crate) fn product<T>(mut self) -> VmResult<T>
    where
        T: FromValue + CheckedOps,
    {
        let mut product = match vm_try!(self.iter.next()) {
            Some(init) => vm_try!(T::from_value(init)),
            None => T::ONE,
        };

        while let Some(v) = vm_try!(self.iter.next()) {
            let v = vm_try!(T::from_value(v));

            let Some(out) = product.checked_mul(v) else {
                return VmResult::err(VmErrorKind::Overflow);
            };

            product = out;
        }

        VmResult::Ok(product)
    }

    #[inline]
    pub(crate) fn sum<T>(mut self) -> VmResult<T>
    where
        T: FromValue + CheckedOps,
    {
        let mut sum = match vm_try!(self.iter.next()) {
            Some(init) => vm_try!(T::from_value(init)),
            None => T::ZERO,
        };

        while let Some(v) = vm_try!(self.next()) {
            let v = vm_try!(T::from_value(v));

            let Some(out) = sum.checked_add(v) else {
                return VmResult::err(VmErrorKind::Overflow);
            };

            sum = out;
        }

        VmResult::Ok(sum)
    }
}

impl fmt::Debug for RuntimeIterator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.iter, f)
    }
}

pub(crate) trait CheckedOps: Sized {
    const ONE: Self;
    const ZERO: Self;

    fn checked_add(self, value: Self) -> Option<Self>;
    fn checked_mul(self, value: Self) -> Option<Self>;
}

impl CheckedOps for u8 {
    const ONE: Self = 1;
    const ZERO: Self = 0;

    #[inline]
    fn checked_add(self, value: Self) -> Option<Self> {
        u8::checked_add(self, value)
    }

    #[inline]
    fn checked_mul(self, value: Self) -> Option<Self> {
        u8::checked_mul(self, value)
    }
}

impl CheckedOps for i64 {
    const ONE: Self = 1;
    const ZERO: Self = 0;

    #[inline]
    fn checked_add(self, value: Self) -> Option<Self> {
        i64::checked_add(self, value)
    }

    #[inline]
    fn checked_mul(self, value: Self) -> Option<Self> {
        i64::checked_mul(self, value)
    }
}

impl CheckedOps for f64 {
    const ONE: Self = 1.0;
    const ZERO: Self = 0.0;

    #[inline]
    fn checked_add(self, value: Self) -> Option<Self> {
        Some(self + value)
    }

    #[inline]
    fn checked_mul(self, value: Self) -> Option<Self> {
        Some(self * value)
    }
}
