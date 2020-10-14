use crate::{FromValue, Function, ToValue, Value, VmError};
use std::iter;
use std::vec;

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
    inner: Inner,
}

crate::__internal_impl_any!(Iterator);

impl Iterator {
    /// Construct a new owning iterator.
    ///
    /// The name is only intended to identify the iterator in case of errors.
    pub fn from<T>(name: &'static str, iter: T) -> Self
    where
        T: IteratorTrait,
    {
        Self {
            inner: Inner::Iterator(Box::new(IteratorObj { name, iter })),
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
            inner: Inner::DoubleEndedIterator(Box::new(IteratorObj { name, iter })),
        }
    }

    /// Get the size hint for the iterator.
    pub fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    /// Get the next value out of the iterator.
    pub fn next(&mut self) -> Result<Option<Value>, VmError> {
        self.inner.next()
    }

    /// Get the next back value out of the iterator.
    pub fn next_back(&mut self) -> Result<Option<Value>, VmError> {
        self.inner.next_back()
    }

    /// Enumerate the iterator.
    pub fn enumerate(self) -> Self {
        Self {
            inner: Inner::Enumerate(Box::new(Enumerate {
                inner: self.inner,
                count: 0,
            })),
        }
    }

    /// Map the iterator using the given function.
    pub fn map(self, map: Function) -> Self {
        Self {
            inner: Inner::Map(Box::new(Map {
                inner: self.inner,
                map,
            })),
        }
    }

    /// Filter the iterator using the given function.
    pub fn filter(self, filter: Function) -> Self {
        Self {
            inner: Inner::Filter(Box::new(Filter {
                inner: self.inner,
                filter,
            })),
        }
    }

    /// Chain this iterator with another.
    pub fn chain(self, other: Self) -> Self {
        Self {
            inner: Inner::Chain(Box::new(Chain {
                a: Some(self.inner),
                b: Some(other.inner),
            })),
        }
    }

    /// Map the iterator using the given function.
    pub fn rev(self) -> Result<Self, VmError> {
        if !self.inner.is_double_ended() {
            let name = self.inner.name();

            return Err(VmError::panic(format!(
                "`{}` is not a double-ended iterator",
                name
            )));
        }

        Ok(Self {
            inner: match self.inner {
                // NB: reversing a reversed iterator restores the original
                // iterator.
                Inner::Rev(inner) => *inner,
                inner => Inner::Rev(Box::new(inner)),
            },
        })
    }

    /// Take the given number of elements from the iterator.
    pub fn take(self, n: usize) -> Self {
        Self {
            inner: Inner::Take(Box::new(Take {
                inner: self.inner,
                n,
            })),
        }
    }

    /// Create a peekable iterator.
    pub fn peekable(self) -> Self {
        Self {
            inner: match self.inner {
                Inner::Peekable(peekable) => Inner::Peekable(peekable),
                inner => Inner::Peekable(Box::new(Peekable {
                    inner,
                    peeked: None,
                })),
            },
        }
    }

    /// Peek the next element if supported.
    pub fn peek(&mut self) -> Result<Option<Value>, VmError> {
        match &mut self.inner {
            Inner::Peekable(peekable) => peekable.peek(),
            _ => Err(VmError::panic(format!(
                "`{}` is not a peekable iterator",
                self.inner.name()
            ))),
        }
    }

    /// Collect results from the iterator.
    pub fn collect<T>(mut self) -> Result<vec::Vec<T>, VmError>
    where
        T: FromValue,
    {
        let (cap, _) = self.inner.size_hint();
        let mut vec = vec::Vec::with_capacity(cap);

        while let Some(value) = self.next()? {
            vec.push(T::from_value(value)?);
        }

        Ok(vec)
    }
}

enum Inner {
    Iterator(Box<IteratorObj<dyn IteratorTrait>>),
    DoubleEndedIterator(Box<IteratorObj<dyn DoubleEndedIteratorTrait>>),
    Map(Box<Map>),
    Filter(Box<Filter>),
    Rev(Box<Inner>),
    Chain(Box<Chain>),
    Enumerate(Box<Enumerate>),
    Take(Box<Take>),
    Peekable(Box<Peekable>),
}

impl Inner {
    /// Test if this iterator is double-ended.
    fn name(&self) -> &'static str {
        match self {
            Inner::Iterator(iter) => iter.name,
            Inner::DoubleEndedIterator(iter) => iter.name,
            Inner::Map(map) => map.inner.name(),
            Inner::Filter(filter) => filter.inner.name(),
            Inner::Rev(inner) => inner.name(),
            Inner::Chain(..) => "std::iter::Chain",
            Inner::Enumerate(enumerate) => enumerate.inner.name(),
            Inner::Take(take) => take.inner.name(),
            Inner::Peekable(peekable) => peekable.inner.name(),
        }
    }

    /// Test if this iterator is double-ended.
    fn is_double_ended(&self) -> bool {
        match self {
            Inner::Iterator(..) => false,
            Inner::DoubleEndedIterator(..) => true,
            Inner::Map(map) => map.inner.is_double_ended(),
            Inner::Filter(filter) => filter.inner.is_double_ended(),
            Inner::Rev(..) => true,
            Inner::Chain(chain) => chain.is_double_ended(),
            Inner::Enumerate(enumerate) => enumerate.inner.is_double_ended(),
            Inner::Take(take) => take.inner.is_double_ended(),
            Inner::Peekable(peekable) => peekable.inner.is_double_ended(),
        }
    }

    /// Get the length of the iterator if it is an exact length iterator.
    fn len(&self) -> Result<usize, VmError> {
        let (lower, upper) = self.size_hint();

        if !matches!(upper, Some(upper) if lower == upper) {
            return Err(VmError::panic(format!(
                "`{}` is not an exact-sized iterator",
                self.name()
            )));
        }

        Ok(lower)
    }

    /// The length of the remaining iterator.
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Inner::Iterator(iter) => iter.iter.size_hint(),
            Inner::DoubleEndedIterator(iter) => iter.iter.size_hint(),
            Inner::Map(map) => map.inner.size_hint(),
            Inner::Filter(filter) => filter.inner.size_hint(),
            Inner::Rev(inner) => inner.size_hint(),
            Inner::Chain(chain) => chain.size_hint(),
            Inner::Enumerate(enumerate) => enumerate.inner.size_hint(),
            Inner::Take(take) => take.inner.size_hint(),
            Inner::Peekable(peekable) => peekable.size_hint(),
        }
    }

    fn next(&mut self) -> Result<Option<Value>, VmError> {
        match self {
            Self::Iterator(owned) => owned.iter.next(),
            Self::DoubleEndedIterator(owned) => owned.iter.next(),
            Self::Map(map) => map.advance(Self::next),
            Self::Filter(filter) => filter.advance(Self::next),
            Self::Rev(rev) => rev.next_back(),
            Inner::Chain(chain) => chain.next(),
            Inner::Enumerate(enumerate) => enumerate.next(),
            Inner::Take(take) => take.next(),
            Inner::Peekable(peekable) => peekable.next(),
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
            Self::DoubleEndedIterator(owned) => owned.iter.next_back(),
            Self::Map(map) => map.advance(Self::next_back),
            Self::Filter(filter) => filter.advance(Self::next_back),
            Self::Rev(rev) => rev.next(),
            Inner::Chain(chain) => chain.next_back(),
            Inner::Enumerate(enumerate) => enumerate.next_back(),
            Inner::Take(take) => take.next_back(),
            Inner::Peekable(peekable) => peekable.next_back(),
        }
    }
}

struct Map {
    inner: Inner,
    map: Function,
}

impl Map {
    fn advance(
        &mut self,
        advance: impl FnOnce(&mut Inner) -> Result<Option<Value>, VmError>,
    ) -> Result<Option<Value>, VmError> {
        if let Some(value) = advance(&mut self.inner)? {
            let out = self.map.call::<_, Value>((value,))?;
            return Ok(Some(out));
        }

        Ok(None)
    }
}

struct Filter {
    inner: Inner,
    filter: Function,
}

impl Filter {
    fn advance(
        &mut self,
        advance: impl Fn(&mut Inner) -> Result<Option<Value>, VmError>,
    ) -> Result<Option<Value>, VmError> {
        while let Some(value) = advance(&mut self.inner)? {
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
/// arguments that can be used in [Iterator::from_iterator].
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

struct Chain {
    a: Option<Inner>,
    b: Option<Inner>,
}

impl Chain {
    /// Determine if the chain is double ended.
    ///
    /// It is only double ended if all remaining iterators are double ended.
    fn is_double_ended(&self) -> bool {
        self.a.as_ref().map(Inner::is_double_ended).unwrap_or(true)
            && self.b.as_ref().map(Inner::is_double_ended).unwrap_or(true)
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

struct Enumerate {
    inner: Inner,
    count: usize,
}

impl Enumerate {
    #[inline]
    fn next(&mut self) -> Result<Option<Value>, VmError> {
        let value = match self.inner.next()? {
            Some(value) => value,
            None => return Ok(None),
        };

        let index = self.count;
        self.count = self.count.saturating_add(1);
        Ok(Some((index, value).to_value()?))
    }

    #[inline]
    fn next_back(&mut self) -> Result<Option<Value>, VmError> {
        let value = match self.inner.next_back()? {
            Some(value) => value,
            None => return Ok(None),
        };

        let len = self.inner.len()?;
        Ok(Some((self.count + len, value).to_value()?))
    }
}

struct Take {
    inner: Inner,
    n: usize,
}

impl Take {
    #[inline]
    fn next(&mut self) -> Result<Option<Value>, VmError> {
        if self.n == 0 {
            return Ok(None);
        }

        self.n -= 1;
        self.inner.next()
    }

    #[inline]
    fn next_back(&mut self) -> Result<Option<Value>, VmError> {
        if self.n == 0 {
            return Ok(None);
        }

        self.n -= 1;
        self.inner.next_back()
    }
}

struct Peekable {
    inner: Inner,
    peeked: Option<Option<Value>>,
}

impl Peekable {
    #[inline]
    fn next(&mut self) -> Result<Option<Value>, VmError> {
        match self.peeked.take() {
            Some(v) => Ok(v),
            None => self.inner.next(),
        }
    }

    #[inline]
    fn next_back(&mut self) -> Result<Option<Value>, VmError> {
        match self.peeked.as_mut() {
            Some(v @ Some(_)) => Ok(self.inner.next_back()?.or_else(|| v.take())),
            Some(None) => Ok(None),
            None => self.inner.next_back(),
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let peek_len = match self.peeked {
            Some(None) => return (0, Some(0)),
            Some(Some(_)) => 1,
            None => 0,
        };
        let (lo, hi) = self.inner.size_hint();
        let lo = lo.saturating_add(peek_len);
        let hi = match hi {
            Some(x) => x.checked_add(peek_len),
            None => None,
        };
        (lo, hi)
    }

    #[inline]
    fn peek(&mut self) -> Result<Option<Value>, VmError> {
        if let Some(value) = &self.peeked {
            return Ok(value.clone());
        }

        let value = self.inner.next()?;
        self.peeked = Some(value.clone());
        Ok(value)
    }
}
