use crate::{
    FromValue, Function, InstallWith, Mut, Named, RawMut, RawRef, RawStr, Ref, ToValue,
    UnsafeFromValue, Value, VmError, VmErrorKind,
};
use std::fmt;
use std::iter;
use std::vec;

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
    fn next(&mut self) -> Result<Option<Value>, VmError>;

    /// Get the next back value out of the iterator.
    fn next_back(&mut self) -> Result<Option<Value>, VmError>;

    /// Get the length of the iterator if it is an exact length iterator.
    #[inline]
    fn len(&self) -> Result<usize, VmError> {
        let (lower, upper) = self.size_hint();

        if !matches!(upper, Some(upper) if lower == upper) {
            return Err(VmError::panic(format!(
                "`{:?}` is not an exact-sized iterator",
                self
            )));
        }

        Ok(lower)
    }
}

/// Fuse the iterator if the expression is `None`.
macro_rules! fuse {
    ($self:ident . $iter:ident . $($call:tt)+) => {
        match $self.$iter {
            Some(ref mut iter) => match iter.$($call)+ {
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
            Some(ref mut iter) => iter.$($call)+,
            None => None,
        }
    };
}

/// An owning iterator.
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

    /// Creates an iterator that yields nothing.
    pub fn empty() -> Self {
        Self {
            iter: IterRepr::Empty,
        }
    }

    /// Creates an iterator that yields an element exactly once.
    pub fn once(value: Value) -> Self {
        Self {
            iter: IterRepr::Once(Some(value)),
        }
    }

    /// Get the size hint for the iterator.
    pub fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    /// Get the next value out of the iterator.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<Option<Value>, VmError> {
        self.iter.next()
    }

    /// Get the next back value out of the iterator.
    pub fn next_back(&mut self) -> Result<Option<Value>, VmError> {
        self.iter.next_back()
    }

    /// Enumerate the iterator.
    pub fn enumerate(self) -> Self {
        Self {
            iter: IterRepr::Enumerate(Box::new(Enumerate {
                iter: self.iter,
                count: 0,
            })),
        }
    }

    /// Map the iterator using the given function.
    pub fn map(self, map: Function) -> Self {
        Self {
            iter: IterRepr::Map(Box::new(Map {
                iter: self.iter,
                map,
            })),
        }
    }

    /// Map and flatten the iterator using the given function.
    pub fn flat_map(self, map: Function) -> Self {
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

    /// Filter the iterator using the given function.
    pub fn filter(self, filter: Function) -> Self {
        Self {
            iter: IterRepr::Filter(Box::new(Filter {
                iter: self.iter,
                filter,
            })),
        }
    }

    /// Find the first matching value in the iterator using the given function.
    pub fn find(mut self, find: Function) -> Result<Option<Value>, VmError> {
        while let Some(value) = self.next()? {
            let result = find.call::<_, bool>((value.clone(),))?;
            if result {
                return Ok(Some(value.clone()));
            }
        }

        Ok(None)
    }

    /// Test if all entries in the iterator matches the given predicate.
    pub fn all(mut self, find: Function) -> Result<bool, VmError> {
        while let Some(value) = self.next()? {
            let result = find.call::<_, bool>((value.clone(),))?;

            if !result {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Test if any entry in the iterator matches the given predicate.
    pub fn any(mut self, find: Function) -> Result<bool, VmError> {
        while let Some(value) = self.next()? {
            if find.call::<_, bool>((value.clone(),))? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Chain this iterator with another.
    pub fn chain(self, other: Value) -> Result<Self, VmError> {
        let other = other.into_iter()?;

        Ok(Self {
            iter: IterRepr::Chain(Box::new(Chain {
                a: Some(self.iter),
                b: Some(other.iter),
            })),
        })
    }

    /// Chain this iterator with another.
    pub fn chain_raw(self, other: Self) -> Result<Self, VmError> {
        Ok(Self {
            iter: IterRepr::Chain(Box::new(Chain {
                a: Some(self.iter),
                b: Some(other.iter),
            })),
        })
    }

    /// Map the iterator using the given function.
    pub fn rev(self) -> Result<Self, VmError> {
        if !self.iter.is_double_ended() {
            return Err(VmError::panic(format!(
                "`{:?}` is not a double-ended iterator",
                self
            )));
        }

        Ok(Self {
            iter: match self.iter {
                // NB: reversing a reversed iterator restores the original
                // iterator.
                IterRepr::Rev(rev) => rev.iter,
                iter => IterRepr::Rev(Box::new(Rev { iter })),
            },
        })
    }

    /// Skip over the given number of elements from the iterator.
    pub fn skip(self, n: usize) -> Self {
        Self {
            iter: IterRepr::Skip(Box::new(Skip { iter: self.iter, n })),
        }
    }

    /// Take the given number of elements from the iterator.
    pub fn take(self, n: usize) -> Self {
        Self {
            iter: IterRepr::Take(Box::new(Take { iter: self.iter, n })),
        }
    }

    /// Count the number of elements remaining in the iterator.
    pub fn count(&mut self) -> Result<usize, VmError> {
        let mut c = 0;

        while self.iter.next()?.is_some() {
            c += 1;
        }

        Ok(c)
    }

    /// Create a peekable iterator.
    pub fn peekable(self) -> Self {
        Self {
            iter: match self.iter {
                IterRepr::Peekable(peekable) => IterRepr::Peekable(peekable),
                iter => IterRepr::Peekable(Box::new(Peekable { iter, peeked: None })),
            },
        }
    }

    /// Peek the next element if supported.
    pub fn peek(&mut self) -> Result<Option<Value>, VmError> {
        match &mut self.iter {
            IterRepr::Peekable(peekable) => peekable.peek(),
            _ => Err(VmError::panic(format!(
                "`{:?}` is not a peekable iterator",
                self.iter
            ))),
        }
    }

    /// Collect results from the iterator.
    pub fn collect<T>(mut self) -> Result<vec::Vec<T>, VmError>
    where
        T: FromValue,
    {
        let (cap, _) = self.iter.size_hint();
        let mut vec = vec::Vec::with_capacity(cap);

        while let Some(value) = self.next()? {
            vec.push(T::from_value(value)?);
        }

        Ok(vec)
    }

    /// Integrate over the iterator, using accumulator as the initial value and
    /// then forwarding the result of each stage.
    pub fn fold(mut self, mut accumulator: Value, f: Function) -> Result<Value, VmError> {
        while let Some(value) = self.next()? {
            accumulator = f.call::<_, Value>((accumulator, value.clone()))?
        }

        Ok(accumulator)
    }

    /// Compute the product under the assumption of a homogeonous iterator of type T.
    pub fn product(self) -> Result<Value, VmError> {
        let product = Product { iter: self.iter };
        product.resolve()
    }

    /// Compute the sum under the assumption of a homogeonous iterator of type T.
    pub fn sum(self) -> Result<Value, VmError> {
        let sum = Sum { iter: self.iter };
        sum.resolve()
    }
}

impl fmt::Debug for Iterator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.iter, f)
    }
}

impl Named for Iterator {
    const BASE_NAME: RawStr = RawStr::from_str("Iterator");
}

impl InstallWith for Iterator {}

impl FromValue for Iterator {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_iterator()?.take()?)
    }
}

impl<'a> UnsafeFromValue for &'a Iterator {
    type Output = *const Iterator;
    type Guard = RawRef;

    fn from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let iterator = value.into_iterator()?;
        Ok(Ref::into_raw(iterator.into_ref()?))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &*output
    }
}

impl<'a> UnsafeFromValue for &'a mut Iterator {
    type Output = *mut Iterator;
    type Guard = RawMut;

    fn from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let iterator = value.into_iterator()?;
        Ok(Mut::into_raw(iterator.into_mut()?))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &mut *output
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

    fn next(&mut self) -> Result<Option<Value>, VmError> {
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
            Self::Empty => Ok(None),
            Self::Once(v) => Ok(v.take()),
        }
    }

    fn next_back(&mut self) -> Result<Option<Value>, VmError> {
        match self {
            Self::Iterator(iter) => {
                return Err(VmError::panic(format!(
                    "`{}` is not a double-ended iterator",
                    iter.name
                )));
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
            Self::Empty => Ok(None),
            Self::Once(v) => Ok(v.take()),
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

    fn next(&mut self) -> Result<Option<Value>, VmError> {
        if let Some(value) = self.iter.next()? {
            let out = self.map.call::<_, Value>((value,))?;
            return Ok(Some(out));
        }

        Ok(None)
    }

    fn next_back(&mut self) -> Result<Option<Value>, VmError> {
        if let Some(value) = self.iter.next_back()? {
            let out = self.map.call::<_, Value>((value,))?;
            return Ok(Some(out));
        }

        Ok(None)
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

    fn next(&mut self) -> Result<Option<Value>, VmError> {
        loop {
            if let Some(iter) = &mut self.frontiter {
                match iter.next()? {
                    None => self.frontiter = None,
                    item @ Some(_) => return Ok(item),
                }
            }

            match self.map.next()? {
                None => {
                    return Ok(match &mut self.backiter {
                        Some(backiter) => backiter.next()?,
                        None => None,
                    })
                }
                Some(value) => {
                    let iterator = value.into_iter()?;
                    self.frontiter = Some(iterator.iter)
                }
            }
        }
    }

    fn next_back(&mut self) -> Result<Option<Value>, VmError> {
        loop {
            if let Some(ref mut iter) = self.backiter {
                match iter.next_back()? {
                    None => self.backiter = None,
                    item @ Some(_) => return Ok(item),
                }
            }

            match self.map.next_back()? {
                None => {
                    return Ok(match &mut self.frontiter {
                        Some(frontiter) => frontiter.next_back()?,
                        None => None,
                    })
                }
                Some(value) => {
                    let iterator = value.into_iter()?;
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
        self.iter.size_hint()
    }

    fn next(&mut self) -> Result<Option<Value>, VmError> {
        while let Some(value) = self.iter.next()? {
            if self.filter.call::<_, bool>((value.clone(),))? {
                return Ok(Some(value));
            }
        }

        Ok(None)
    }

    fn next_back(&mut self) -> Result<Option<Value>, VmError> {
        while let Some(value) = self.iter.next_back()? {
            if self.filter.call::<_, bool>((value.clone(),))? {
                return Ok(Some(value));
            }
        }

        Ok(None)
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
    fn next(&mut self) -> Result<Option<Value>, VmError>;
}

impl<T> IteratorTrait for T
where
    T: 'static,
    T: iter::Iterator,
    T::Item: ToValue,
{
    fn size_hint(&self) -> (usize, Option<usize>) {
        iter::Iterator::size_hint(self)
    }

    fn next(&mut self) -> Result<Option<Value>, VmError> {
        let value = match iter::Iterator::next(self) {
            Some(value) => value,
            None => return Ok(None),
        };

        Ok(Some(value.to_value()?))
    }
}

/// The trait for interacting with an iterator.
///
/// This has a blanket implementation, and is primarily used to restrict the
/// arguments that can be used in [Iterator::from_double_ended].
pub trait DoubleEndedIteratorTrait: IteratorTrait {
    /// Get the next back value out of the iterator.
    fn next_back(&mut self) -> Result<Option<Value>, VmError>;
}

impl<T> DoubleEndedIteratorTrait for T
where
    T: 'static,
    T: iter::DoubleEndedIterator,
    T::Item: ToValue,
{
    fn next_back(&mut self) -> Result<Option<Value>, VmError> {
        let value = match iter::DoubleEndedIterator::next_back(self) {
            Some(value) => value,
            None => return Ok(None),
        };

        Ok(Some(value.to_value()?))
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
    fn next(&mut self) -> Result<Option<Value>, VmError> {
        Ok(match fuse!(self.a.next()?) {
            None => maybe!(self.b.next()?),
            item => item,
        })
    }

    #[inline]
    fn next_back(&mut self) -> Result<Option<Value>, VmError> {
        Ok(match fuse!(self.b.next_back()?) {
            None => maybe!(self.a.next_back()?),
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
    fn next(&mut self) -> Result<Option<Value>, VmError> {
        let value = match self.iter.next()? {
            Some(value) => value,
            None => return Ok(None),
        };

        let index = self.count;
        self.count = self.count.saturating_add(1);
        Ok(Some((index, value).to_value()?))
    }

    #[inline]
    fn next_back(&mut self) -> Result<Option<Value>, VmError> {
        let value = match self.iter.next_back()? {
            Some(value) => value,
            None => return Ok(None),
        };

        let len = self.iter.len()?;
        Ok(Some((self.count + len, value).to_value()?))
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
    fn next(&mut self) -> Result<Option<Value>, VmError> {
        self.iter.next_back()
    }

    #[inline]
    fn next_back(&mut self) -> Result<Option<Value>, VmError> {
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
    fn next(&mut self) -> Result<Option<Value>, VmError> {
        if self.n > 0 {
            let old_n = self.n;
            self.n = 0;

            for _ in 0..old_n {
                match self.iter.next()? {
                    Some(..) => (),
                    None => return Ok(None),
                }
            }
        }

        self.iter.next()
    }

    #[inline]
    fn next_back(&mut self) -> Result<Option<Value>, VmError> {
        Ok(if self.len()? > 0 {
            self.iter.next_back()?
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

        let lower = std::cmp::min(lower, self.n);

        let upper = match upper {
            Some(x) if x < self.n => Some(x),
            _ => Some(self.n),
        };

        (lower, upper)
    }

    #[inline]
    fn next(&mut self) -> Result<Option<Value>, VmError> {
        if self.n == 0 {
            return Ok(None);
        }

        self.n -= 1;
        self.iter.next()
    }

    #[inline]
    fn next_back(&mut self) -> Result<Option<Value>, VmError> {
        if self.n == 0 {
            return Ok(None);
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
    fn peek(&mut self) -> Result<Option<Value>, VmError> {
        if let Some(value) = &self.peeked {
            return Ok(value.clone());
        }

        let value = self.iter.next()?;
        self.peeked = Some(value.clone());
        Ok(value)
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
    fn next(&mut self) -> Result<Option<Value>, VmError> {
        match self.peeked.take() {
            Some(v) => Ok(v),
            None => self.iter.next(),
        }
    }

    #[inline]
    fn next_back(&mut self) -> Result<Option<Value>, VmError> {
        match self.peeked.as_mut() {
            Some(v @ Some(_)) => Ok(self.iter.next_back()?.or_else(|| v.take())),
            Some(None) => Ok(None),
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
    fn next(&mut self) -> Result<Option<Value>, VmError> {
        if let Some(iter) = &mut self.iter {
            if let Some(value) = iter.next()? {
                return Ok(Some(value));
            }

            self.iter = None;
        }

        Ok(None)
    }

    #[inline]
    fn next_back(&mut self) -> Result<Option<Value>, VmError> {
        if let Some(iter) = &mut self.iter {
            if let Some(value) = iter.next_back()? {
                return Ok(Some(value));
            }

            self.iter = None;
        }

        Ok(None)
    }
}

struct Product<I>
where
    I: RuneIterator,
{
    iter: I,
}

impl<I> Product<I>
where
    I: RuneIterator,
{
    fn next<T: FromValue>(&mut self) -> Result<Option<T>, VmError> {
        self.iter.next()?.map(T::from_value).transpose()
    }

    fn resolve_internal_simple<T: FromValue + std::ops::Mul<Output = T>>(
        &mut self,
        first: T,
    ) -> Result<T, VmError> {
        let mut product = first;
        while let Some(v) = self.next()? {
            product = product * v;
        }

        Ok(product)
    }

    fn resolve(mut self) -> Result<Value, VmError> {
        match self.iter.next()? {
            Some(v) => match v {
                Value::Byte(v) => Ok(Value::Byte(self.resolve_internal_simple(v)?)),
                Value::Integer(v) => Ok(Value::Integer(self.resolve_internal_simple(v)?)),
                Value::Float(v) => Ok(Value::Float(self.resolve_internal_simple(v)?)),
                _ => Err(VmError::from(VmErrorKind::UnsupportedBinaryOperation {
                    op: "*",
                    lhs: v.type_info()?,
                    rhs: v.type_info()?,
                })),
            },
            None => Err(VmError::panic(
                "cannot take the product of an empty iterator",
            )),
        }
    }
}

struct Sum<I>
where
    I: RuneIterator,
{
    iter: I,
}

impl<I> Sum<I>
where
    I: RuneIterator,
{
    fn next<T: FromValue>(&mut self) -> Result<Option<T>, VmError> {
        self.iter.next()?.map(T::from_value).transpose()
    }

    fn resolve_internal_simple<T: FromValue + std::ops::Add<Output = T>>(
        &mut self,
        first: T,
    ) -> Result<T, VmError> {
        let mut sum = first;
        while let Some(v) = self.next()? {
            sum = sum + v;
        }

        Ok(sum)
    }

    fn resolve(mut self) -> Result<Value, VmError> {
        match self.iter.next()? {
            Some(v) => match v {
                Value::Byte(v) => Ok(Value::Byte(self.resolve_internal_simple(v)?)),
                Value::Integer(v) => Ok(Value::Integer(self.resolve_internal_simple(v)?)),
                Value::Float(v) => Ok(Value::Float(self.resolve_internal_simple(v)?)),
                _ => Err(VmError::from(VmErrorKind::UnsupportedBinaryOperation {
                    op: "+",
                    lhs: v.type_info()?,
                    rhs: v.type_info()?,
                })),
            },
            None => Err(VmError::panic("cannot take the sum of an empty iterator")),
        }
    }
}
