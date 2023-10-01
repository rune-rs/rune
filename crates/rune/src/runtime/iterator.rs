use core::cmp;
use core::fmt;
use core::iter;

use rust_alloc::boxed::Box;

use crate as rune;
use crate::alloc;
use crate::alloc::prelude::*;
use crate::runtime::{FromValue, Function, Panic, ToValue, Value, VmErrorKind, VmResult};
use crate::Any;

// Note: A fair amount of code in this module is duplicated from the Rust
// project under the MIT license.
//
// https://github.com/rust-lang/rust
//
// Copyright 2014-2020 The Rust Project Developers

/// Internal iterator trait used to build useful internal iterator abstractions,
/// like [Fuse].
trait RuneIterator: fmt::Debug {
    /// Test if the iterator is double-ended.
    fn is_double_ended(&self) -> bool;

    /// The length of the remaining iterator.
    fn size_hint(&self) -> (usize, Option<usize>);

    /// Get the next value out of the iterator.
    fn next(&mut self) -> VmResult<Option<Value>>;

    /// Get the next back value out of the iterator.
    fn next_back(&mut self) -> VmResult<Option<Value>>;

    /// Get the length of the iterator if it is an exact length iterator.
    #[inline]
    fn len(&self) -> VmResult<usize> {
        let (lower, upper) = self.size_hint();

        if !matches!(upper, Some(upper) if lower == upper) {
            return VmResult::panic(format!("`{:?}` is not an exact-sized iterator", self));
        }

        VmResult::Ok(lower)
    }
}

/// Fuse the iterator if the expression is `None`.
macro_rules! fuse {
    ($self:ident . $iter:ident . $($call:tt)+) => {
        match $self.$iter {
            Some(ref mut iter) => match vm_try!(iter.$($call)+) {
                None => {
                    $self.$iter = None;
                    None
                }
                item => item,
            },
            None => None,
        }
    };
}

/// Try an iterator method without fusing,
/// like an inline `.as_mut().and_then(...)`
macro_rules! maybe {
    ($self:ident . $iter:ident . $($call:tt)+) => {
        match $self.$iter {
            Some(ref mut iter) => vm_try!(iter.$($call)+),
            None => None,
        }
    };
}

/// An owning iterator.
#[derive(Any)]
#[rune(builtin, static_type = ITERATOR_TYPE, from_value = Value::into_iterator)]
pub struct Iterator {
    iter: IterRepr,
}

impl Iterator {
    /// Construct a new owning iterator.
    ///
    /// The name is only intended to identify the iterator in case of errors.
    pub fn from<T>(name: &'static str, iter: T) -> Self
    where
        T: IteratorTrait,
    {
        Self {
            iter: IterRepr::Iterator(Box::new(IteratorObj { name, iter })),
        }
    }

    /// Construct a new double-ended owning iterator, with a human-readable
    /// name.
    ///
    /// The name is only intended to identify the iterator in case of errors.
    pub fn from_double_ended<T>(name: &'static str, iter: T) -> Self
    where
        T: DoubleEndedIteratorTrait,
    {
        Self {
            iter: IterRepr::DoubleEndedIterator(Box::new(IteratorObj { name, iter })),
        }
    }

    #[inline]
    pub(crate) fn empty() -> Self {
        Self {
            iter: IterRepr::Empty,
        }
    }

    #[inline]
    pub(crate) fn once(value: Value) -> Self {
        Self {
            iter: IterRepr::Once(Some(value)),
        }
    }

    #[inline]
    pub(crate) fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    #[inline]
    pub(crate) fn next(&mut self) -> VmResult<Option<Value>> {
        self.iter.next()
    }

    #[inline]
    pub(crate) fn next_back(&mut self) -> VmResult<Option<Value>> {
        self.iter.next_back()
    }

    #[inline]
    pub(crate) fn enumerate(self) -> Self {
        Self {
            iter: IterRepr::Enumerate(Box::new(Enumerate {
                iter: self.iter,
                count: 0,
            })),
        }
    }

    #[inline]
    pub(crate) fn filter(self, filter: Function) -> Self {
        Self {
            iter: IterRepr::Filter(Box::new(Filter {
                iter: self.iter,
                filter,
            })),
        }
    }

    #[inline]
    pub(crate) fn map(self, map: Function) -> Self {
        Self {
            iter: IterRepr::Map(Box::new(Map {
                iter: self.iter,
                map,
            })),
        }
    }

    #[inline]
    pub(crate) fn flat_map(self, map: Function) -> Self {
        Self {
            iter: IterRepr::FlatMap(Box::new(FlatMap {
                map: Fuse::new(Map {
                    iter: self.iter,
                    map,
                }),
                frontiter: None,
                backiter: None,
            })),
        }
    }

    #[inline]
    pub(crate) fn find(&mut self, find: Function) -> VmResult<Option<Value>> {
        while let Some(value) = vm_try!(self.next()) {
            if vm_try!(find.call::<_, bool>((value.clone(),))) {
                return VmResult::Ok(Some(value));
            }
        }

        VmResult::Ok(None)
    }

    #[inline]
    pub(crate) fn all(&mut self, find: Function) -> VmResult<bool> {
        while let Some(value) = vm_try!(self.next()) {
            let result = vm_try!(find.call::<_, bool>((value.clone(),)));

            if !result {
                return VmResult::Ok(false);
            }
        }

        VmResult::Ok(true)
    }

    #[inline]
    pub(crate) fn any(&mut self, find: Function) -> VmResult<bool> {
        while let Some(value) = vm_try!(self.next()) {
            if vm_try!(find.call::<_, bool>((value.clone(),))) {
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
        if !self.iter.is_double_ended() {
            return VmResult::panic(format!("`{:?}` is not a double-ended iterator", self));
        }

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

        while vm_try!(self.iter.next()).is_some() {
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
            accumulator = vm_try!(f.call::<_, Value>((accumulator, value.clone())));
        }

        VmResult::Ok(accumulator)
    }

    #[inline]
    pub(crate) fn reduce(mut self, f: Function) -> VmResult<Option<Value>> {
        let Some(mut accumulator) = vm_try!(self.next()) else {
            return VmResult::Ok(None);
        };

        while let Some(value) = vm_try!(self.next()) {
            accumulator = vm_try!(f.call::<_, Value>((accumulator, value.clone())));
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

impl fmt::Debug for Iterator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.iter, f)
    }
}

/// The inner representation of an [Iterator]. It handles all the necessary
/// dynamic dispatch to support dynamic iterators.
enum IterRepr {
    Iterator(Box<IteratorObj<dyn IteratorTrait>>),
    DoubleEndedIterator(Box<IteratorObj<dyn DoubleEndedIteratorTrait>>),
    Map(Box<Map<Self>>),
    FlatMap(Box<FlatMap<Map<Self>>>),
    Filter(Box<Filter<Self>>),
    Rev(Box<Rev<Self>>),
    Chain(Box<Chain<Self, Self>>),
    Enumerate(Box<Enumerate<Self>>),
    Skip(Box<Skip<Self>>),
    Take(Box<Take<Self>>),
    Peekable(Box<Peekable<Self>>),
    Empty,
    Once(Option<Value>),
}

impl RuneIterator for IterRepr {
    /// Test if this iterator is double-ended.
    fn is_double_ended(&self) -> bool {
        match self {
            Self::Iterator(..) => false,
            Self::DoubleEndedIterator(..) => true,
            Self::Map(iter) => iter.is_double_ended(),
            Self::FlatMap(iter) => iter.is_double_ended(),
            Self::Filter(iter) => iter.is_double_ended(),
            Self::Rev(..) => true,
            Self::Chain(iter) => iter.is_double_ended(),
            Self::Enumerate(iter) => iter.is_double_ended(),
            Self::Skip(iter) => iter.is_double_ended(),
            Self::Take(iter) => iter.is_double_ended(),
            Self::Peekable(iter) => iter.is_double_ended(),
            Self::Empty => true,
            Self::Once(..) => true,
        }
    }

    /// The length of the remaining iterator.
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Iterator(iter) => iter.iter.size_hint(),
            Self::DoubleEndedIterator(iter) => iter.iter.size_hint(),
            Self::Map(iter) => iter.size_hint(),
            Self::FlatMap(iter) => iter.size_hint(),
            Self::Filter(iter) => iter.size_hint(),
            Self::Rev(iter) => iter.size_hint(),
            Self::Chain(iter) => iter.size_hint(),
            Self::Enumerate(iter) => iter.size_hint(),
            Self::Skip(iter) => iter.size_hint(),
            Self::Take(iter) => iter.size_hint(),
            Self::Peekable(iter) => iter.size_hint(),
            Self::Empty => (0, Some(0)),
            Self::Once(..) => (1, Some(1)),
        }
    }

    fn next(&mut self) -> VmResult<Option<Value>> {
        match self {
            Self::Iterator(iter) => iter.iter.next(),
            Self::DoubleEndedIterator(iter) => iter.iter.next(),
            Self::Map(iter) => iter.next(),
            Self::FlatMap(iter) => iter.next(),
            Self::Filter(iter) => iter.next(),
            Self::Rev(iter) => iter.next(),
            Self::Chain(iter) => iter.next(),
            Self::Enumerate(iter) => iter.next(),
            Self::Skip(iter) => iter.next(),
            Self::Take(iter) => iter.next(),
            Self::Peekable(iter) => iter.next(),
            Self::Empty => VmResult::Ok(None),
            Self::Once(v) => VmResult::Ok(v.take()),
        }
    }

    fn next_back(&mut self) -> VmResult<Option<Value>> {
        match self {
            Self::Iterator(iter) => {
                let message = vm_try!(format_args!(
                    "`{}` is not a double-ended iterator",
                    iter.name
                )
                .try_to_string());
                VmResult::err(Panic::custom(message))
            }
            Self::DoubleEndedIterator(iter) => iter.iter.next_back(),
            Self::Map(iter) => iter.next_back(),
            Self::FlatMap(iter) => iter.next_back(),
            Self::Filter(iter) => iter.next_back(),
            Self::Rev(iter) => iter.next_back(),
            Self::Chain(iter) => iter.next_back(),
            Self::Enumerate(iter) => iter.next_back(),
            Self::Skip(iter) => iter.next_back(),
            Self::Take(iter) => iter.next_back(),
            Self::Peekable(iter) => iter.next_back(),
            Self::Empty => VmResult::Ok(None),
            Self::Once(v) => VmResult::Ok(v.take()),
        }
    }
}

impl fmt::Debug for IterRepr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Iterator(iter) => write!(f, "{}", iter.name),
            Self::DoubleEndedIterator(iter) => write!(f, "{}", iter.name),
            Self::Map(iter) => write!(f, "{:?}", iter),
            Self::FlatMap(iter) => write!(f, "{:?}", iter),
            Self::Filter(iter) => write!(f, "{:?}", iter),
            Self::Rev(iter) => write!(f, "{:?}", iter),
            Self::Chain(iter) => write!(f, "{:?}", iter),
            Self::Enumerate(iter) => write!(f, "{:?}", iter),
            Self::Skip(iter) => write!(f, "{:?}", iter),
            Self::Take(iter) => write!(f, "{:?}", iter),
            Self::Peekable(iter) => write!(f, "{:?}", iter),
            Self::Empty => write!(f, "std::iter::Empty"),
            Self::Once(..) => write!(f, "std::iter::Once"),
        }
    }
}

#[derive(Debug)]
struct Map<I> {
    iter: I,
    map: Function,
}

impl<I> RuneIterator for Map<I>
where
    I: RuneIterator,
{
    fn is_double_ended(&self) -> bool {
        self.iter.is_double_ended()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    fn next(&mut self) -> VmResult<Option<Value>> {
        if let Some(value) = vm_try!(self.iter.next()) {
            let out = vm_try!(self.map.call::<_, Value>((value,)));
            return VmResult::Ok(Some(out));
        }

        VmResult::Ok(None)
    }

    fn next_back(&mut self) -> VmResult<Option<Value>> {
        if let Some(value) = vm_try!(self.iter.next_back()) {
            let out = vm_try!(self.map.call::<_, Value>((value,)));
            return VmResult::Ok(Some(out));
        }

        VmResult::Ok(None)
    }
}

#[derive(Debug)]
struct FlatMap<I> {
    map: Fuse<I>,
    frontiter: Option<IterRepr>,
    backiter: Option<IterRepr>,
}

impl<I> RuneIterator for FlatMap<I>
where
    I: RuneIterator,
{
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (flo, fhi) = self
            .frontiter
            .as_ref()
            .map_or((0, Some(0)), IterRepr::size_hint);

        let (blo, bhi) = self
            .backiter
            .as_ref()
            .map_or((0, Some(0)), IterRepr::size_hint);

        let lo = flo.saturating_add(blo);

        match (self.map.size_hint(), fhi, bhi) {
            ((0, Some(0)), Some(a), Some(b)) => (lo, a.checked_add(b)),
            _ => (lo, None),
        }
    }

    fn is_double_ended(&self) -> bool {
        if !self.map.is_double_ended() {
            return false;
        }

        if !matches!(&self.frontiter, Some(iter) if !iter.is_double_ended()) {
            return false;
        }

        if !matches!(&self.backiter, Some(iter) if !iter.is_double_ended()) {
            return false;
        }

        true
    }

    fn next(&mut self) -> VmResult<Option<Value>> {
        loop {
            if let Some(iter) = &mut self.frontiter {
                match vm_try!(iter.next()) {
                    None => self.frontiter = None,
                    item @ Some(_) => return VmResult::Ok(item),
                }
            }

            match vm_try!(self.map.next()) {
                None => {
                    return VmResult::Ok(match &mut self.backiter {
                        Some(backiter) => vm_try!(backiter.next()),
                        None => None,
                    })
                }
                Some(value) => {
                    let iterator = vm_try!(value.into_iter());
                    self.frontiter = Some(iterator.iter)
                }
            }
        }
    }

    fn next_back(&mut self) -> VmResult<Option<Value>> {
        loop {
            if let Some(ref mut iter) = self.backiter {
                match vm_try!(iter.next_back()) {
                    None => self.backiter = None,
                    item @ Some(_) => return VmResult::Ok(item),
                }
            }

            match vm_try!(self.map.next_back()) {
                None => {
                    return VmResult::Ok(match &mut self.frontiter {
                        Some(frontiter) => vm_try!(frontiter.next_back()),
                        None => None,
                    })
                }
                Some(value) => {
                    let iterator = vm_try!(value.into_iter());
                    self.backiter = Some(iterator.iter);
                }
            }
        }
    }
}

#[derive(Debug)]
struct Filter<I> {
    iter: I,
    filter: Function,
}

impl<I> RuneIterator for Filter<I>
where
    I: RuneIterator,
{
    fn is_double_ended(&self) -> bool {
        self.iter.is_double_ended()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.iter.size_hint();
        (0, upper)
    }

    fn next(&mut self) -> VmResult<Option<Value>> {
        while let Some(value) = vm_try!(self.iter.next()) {
            if vm_try!(self.filter.call::<_, bool>((value.clone(),))) {
                return VmResult::Ok(Some(value));
            }
        }

        VmResult::Ok(None)
    }

    fn next_back(&mut self) -> VmResult<Option<Value>> {
        while let Some(value) = vm_try!(self.iter.next_back()) {
            if vm_try!(self.filter.call::<_, bool>((value.clone(),))) {
                return VmResult::Ok(Some(value));
            }
        }

        VmResult::Ok(None)
    }
}

/// The trait for interacting with an iterator.
///
/// This has a blanket implementation, and is primarily used to restrict the
/// arguments that can be used in [Iterator::from].
pub trait IteratorTrait: 'static {
    /// Size hint of the iterator.
    fn size_hint(&self) -> (usize, Option<usize>);

    /// Get the next value out of the iterator.
    fn next(&mut self) -> VmResult<Option<Value>>;
}

impl<T> IteratorTrait for T
where
    T: 'static,
    T: iter::Iterator,
    T::Item: ToValue,
{
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        iter::Iterator::size_hint(self)
    }

    fn next(&mut self) -> VmResult<Option<Value>> {
        let value = match iter::Iterator::next(self) {
            Some(value) => value,
            None => return VmResult::Ok(None),
        };

        VmResult::Ok(Some(vm_try!(value.to_value())))
    }
}

/// The trait for interacting with an iterator.
///
/// This has a blanket implementation, and is primarily used to restrict the
/// arguments that can be used in [Iterator::from_double_ended].
pub trait DoubleEndedIteratorTrait: IteratorTrait {
    /// Get the next back value out of the iterator.
    fn next_back(&mut self) -> VmResult<Option<Value>>;
}

impl<T> DoubleEndedIteratorTrait for T
where
    T: 'static,
    T: iter::DoubleEndedIterator,
    T::Item: ToValue,
{
    fn next_back(&mut self) -> VmResult<Option<Value>> {
        let value = match iter::DoubleEndedIterator::next_back(self) {
            Some(value) => value,
            None => return VmResult::Ok(None),
        };

        VmResult::Ok(Some(vm_try!(value.to_value())))
    }
}

struct IteratorObj<T>
where
    T: ?Sized,
{
    name: &'static str,
    iter: T,
}

#[derive(Debug)]
struct Chain<A, B> {
    a: Option<A>,
    b: Option<B>,
}

impl<A, B> RuneIterator for Chain<A, B>
where
    A: RuneIterator,
    B: RuneIterator,
{
    /// Determine if the chain is double ended.
    ///
    /// It is only double ended if all remaining iterators are double ended.

    fn is_double_ended(&self) -> bool {
        !matches!(&self.a, Some(i) if !i.is_double_ended())
            && !matches!(&self.b, Some(i) if !i.is_double_ended())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self {
                a: Some(a),
                b: Some(b),
            } => {
                let (a_lower, a_upper) = a.size_hint();
                let (b_lower, b_upper) = b.size_hint();

                let lower = a_lower.saturating_add(b_lower);

                let upper = match (a_upper, b_upper) {
                    (Some(x), Some(y)) => x.checked_add(y),
                    _ => None,
                };

                (lower, upper)
            }
            Self {
                a: Some(a),
                b: None,
            } => a.size_hint(),
            Self {
                a: None,
                b: Some(b),
            } => b.size_hint(),
            Self { a: None, b: None } => (0, Some(0)),
        }
    }

    #[inline]
    fn next(&mut self) -> VmResult<Option<Value>> {
        VmResult::Ok(match fuse!(self.a.next()) {
            None => maybe!(self.b.next()),
            item => item,
        })
    }

    #[inline]
    fn next_back(&mut self) -> VmResult<Option<Value>> {
        VmResult::Ok(match fuse!(self.b.next_back()) {
            None => maybe!(self.a.next_back()),
            item => item,
        })
    }
}

#[derive(Debug)]
struct Enumerate<I> {
    iter: I,
    count: usize,
}

impl<I> RuneIterator for Enumerate<I>
where
    I: RuneIterator,
{
    fn is_double_ended(&self) -> bool {
        self.iter.is_double_ended()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    #[inline]
    fn next(&mut self) -> VmResult<Option<Value>> {
        let value = match vm_try!(self.iter.next()) {
            Some(value) => value,
            None => return VmResult::Ok(None),
        };

        let index = self.count;
        self.count = self.count.saturating_add(1);
        VmResult::Ok(Some(vm_try!((index, value).to_value())))
    }

    #[inline]
    fn next_back(&mut self) -> VmResult<Option<Value>> {
        let value = match vm_try!(self.iter.next_back()) {
            Some(value) => value,
            None => return VmResult::Ok(None),
        };

        let len = vm_try!(self.iter.len());
        VmResult::Ok(Some(vm_try!((self.count + len, value).to_value())))
    }
}

#[derive(Debug)]
#[repr(transparent)]
struct Rev<I> {
    iter: I,
}

impl<I> RuneIterator for Rev<I>
where
    I: RuneIterator,
{
    #[inline]
    fn is_double_ended(&self) -> bool {
        true
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    #[inline]
    fn next(&mut self) -> VmResult<Option<Value>> {
        self.iter.next_back()
    }

    #[inline]
    fn next_back(&mut self) -> VmResult<Option<Value>> {
        self.iter.next()
    }
}

#[derive(Debug)]
struct Skip<I> {
    iter: I,
    n: usize,
}

impl<I> RuneIterator for Skip<I>
where
    I: RuneIterator,
{
    #[inline]
    fn is_double_ended(&self) -> bool {
        self.iter.is_double_ended()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lower, upper) = self.iter.size_hint();

        let lower = lower.saturating_sub(self.n);
        let upper = upper.map(|x| x.saturating_sub(self.n));

        (lower, upper)
    }

    #[inline]
    fn next(&mut self) -> VmResult<Option<Value>> {
        if self.n > 0 {
            let old_n = self.n;
            self.n = 0;

            for _ in 0..old_n {
                match vm_try!(self.iter.next()) {
                    Some(..) => (),
                    None => return VmResult::Ok(None),
                }
            }
        }

        self.iter.next()
    }

    #[inline]
    fn next_back(&mut self) -> VmResult<Option<Value>> {
        VmResult::Ok(if vm_try!(self.len()) > 0 {
            vm_try!(self.iter.next_back())
        } else {
            None
        })
    }
}

#[derive(Debug)]
struct Take<I> {
    iter: I,
    n: usize,
}

impl<I> RuneIterator for Take<I>
where
    I: RuneIterator,
{
    #[inline]
    fn is_double_ended(&self) -> bool {
        self.iter.is_double_ended()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.n == 0 {
            return (0, Some(0));
        }

        let (lower, upper) = self.iter.size_hint();

        let lower = cmp::min(lower, self.n);

        let upper = match upper {
            Some(x) if x < self.n => Some(x),
            _ => Some(self.n),
        };

        (lower, upper)
    }

    #[inline]
    fn next(&mut self) -> VmResult<Option<Value>> {
        if self.n == 0 {
            return VmResult::Ok(None);
        }

        self.n -= 1;
        self.iter.next()
    }

    #[inline]
    fn next_back(&mut self) -> VmResult<Option<Value>> {
        if self.n == 0 {
            return VmResult::Ok(None);
        }

        self.n -= 1;
        self.iter.next_back()
    }
}

#[derive(Debug)]
struct Peekable<I> {
    iter: I,
    peeked: Option<Option<Value>>,
}

impl<I> Peekable<I>
where
    I: RuneIterator,
{
    #[inline]
    fn peek(&mut self) -> VmResult<Option<Value>> {
        if let Some(value) = &self.peeked {
            return VmResult::Ok(value.clone());
        }

        let value = vm_try!(self.iter.next());
        self.peeked = Some(value.clone());
        VmResult::Ok(value)
    }
}

impl<I> Peekable<I>
where
    I: RuneIterator,
{
    #[inline]
    fn is_double_ended(&self) -> bool {
        self.iter.is_double_ended()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let peek_len = match self.peeked {
            Some(None) => return (0, Some(0)),
            Some(Some(_)) => 1,
            None => 0,
        };
        let (lo, hi) = self.iter.size_hint();
        let lo = lo.saturating_add(peek_len);
        let hi = match hi {
            Some(x) => x.checked_add(peek_len),
            None => None,
        };
        (lo, hi)
    }

    #[inline]
    fn next(&mut self) -> VmResult<Option<Value>> {
        match self.peeked.take() {
            Some(v) => VmResult::Ok(v),
            None => self.iter.next(),
        }
    }

    #[inline]
    fn next_back(&mut self) -> VmResult<Option<Value>> {
        match self.peeked.as_mut() {
            Some(v @ Some(_)) => VmResult::Ok(vm_try!(self.iter.next_back()).or_else(|| v.take())),
            Some(None) => VmResult::Ok(None),
            None => self.iter.next_back(),
        }
    }
}

#[derive(Debug)]
struct Fuse<I> {
    iter: Option<I>,
}

impl<I> Fuse<I> {
    fn new(iter: I) -> Self {
        Self { iter: Some(iter) }
    }
}

impl<I> RuneIterator for Fuse<I>
where
    I: RuneIterator,
{
    #[inline]
    fn is_double_ended(&self) -> bool {
        match &self.iter {
            Some(iter) => iter.is_double_ended(),
            // NB: trivially double-ended since it produces no values.
            None => true,
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.iter {
            Some(iter) => iter.size_hint(),
            None => (0, Some(0)),
        }
    }

    #[inline]
    fn next(&mut self) -> VmResult<Option<Value>> {
        if let Some(iter) = &mut self.iter {
            if let Some(value) = vm_try!(iter.next()) {
                return VmResult::Ok(Some(value));
            }

            self.iter = None;
        }

        VmResult::Ok(None)
    }

    #[inline]
    fn next_back(&mut self) -> VmResult<Option<Value>> {
        if let Some(iter) = &mut self.iter {
            if let Some(value) = vm_try!(iter.next_back()) {
                return VmResult::Ok(Some(value));
            }

            self.iter = None;
        }

        VmResult::Ok(None)
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
