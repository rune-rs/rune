use core::fmt::{self, Write};

use crate as rune;
use crate::no_std::collections;
use crate::no_std::prelude::*;

use crate::runtime::{Iterator, Protocol, Value, VmErrorKind, VmResult};
use crate::{Any, ContextError, Module};

pub(super) fn setup(m: &mut Module) -> Result<(), ContextError> {
    m.ty::<VecDeque>()?.docs([
        "A double-ended queue implemented with a growable ring buffer.",
        "",
        "The \"default\" usage of this type as a queue is to use [`push_back`] to add to",
        "the queue, and [`pop_front`] to remove from the queue. [`extend`] and [`append`]",
        "push onto the back in this manner, and iterating over `VecDeque` goes front",
        "to back.",
        "",
        "A `VecDeque` with a known list of items can be initialized from an array:",
        "",
        "```rune",
        "use std::collections::VecDeque;",
        "",
        "let deq = VecDeque::from([-1, 0, 1]);",
        "```",
        "",
        "[`push_back`]: VecDeque::push_back",
        "[`pop_front`]: VecDeque::pop_front",
        "[`extend`]: VecDeque::extend",
        "[`append`]: VecDeque::append",
    ]);

    m.function_meta(VecDeque::new)?;
    m.function_meta(VecDeque::with_capacity)?;
    m.function_meta(from)?;

    m.function_meta(VecDeque::extend)?;
    m.function_meta(VecDeque::insert)?;
    m.function_meta(VecDeque::iter)?;
    m.function_meta(VecDeque::reserve)?;
    m.function_meta(VecDeque::len)?;
    m.function_meta(VecDeque::capacity)?;
    m.function_meta(VecDeque::front)?;
    m.function_meta(VecDeque::back)?;
    m.function_meta(VecDeque::push_back)?;
    m.function_meta(VecDeque::push_front)?;
    m.function_meta(VecDeque::pop_front)?;
    m.function_meta(VecDeque::pop_back)?;
    m.function_meta(VecDeque::remove)?;
    m.function_meta(VecDeque::rotate_left)?;
    m.function_meta(VecDeque::rotate_right)?;

    m.associated_function(Protocol::INDEX_GET, VecDeque::get)?;
    m.associated_function(Protocol::INDEX_SET, VecDeque::set)?;
    m.associated_function(Protocol::INTO_ITER, VecDeque::__rune_fn__iter)?;
    m.associated_function(Protocol::STRING_DEBUG, VecDeque::string_debug)?;
    m.associated_function(Protocol::EQ, eq)?;
    Ok(())
}

#[derive(Any, Clone, Default)]
#[rune(module = crate, item = ::std::collections)]
pub(crate) struct VecDeque {
    inner: collections::VecDeque<Value>,
}

impl VecDeque {
    /// Creates an empty deque.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::VecDeque;
    ///
    /// let deque = VecDeque::new();
    /// ```
    #[rune::function(path = Self::new)]
    fn new() -> VecDeque {
        Self {
            inner: collections::VecDeque::new(),
        }
    }

    /// Creates an empty deque with space for at least `capacity` elements.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::VecDeque;
    ///
    /// let deque = VecDeque::with_capacity(10);
    /// assert!(deque.capacity() >= 10);
    /// ```
    #[rune::function(path = Self::with_capacity)]
    fn with_capacity(count: usize) -> VecDeque {
        Self {
            inner: collections::VecDeque::with_capacity(count),
        }
    }

    /// Extend this VecDeque with something that implements the [`INTO_ITER`]
    /// protocol.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::VecDeque;
    ///
    /// let deque = VecDeque::new();
    /// deque.extend([1, 2, 3]);
    ///
    /// assert_eq!(Some(1), deque.pop_front());
    /// assert_eq!(Some(3), deque.pop_back());
    /// ```
    #[rune::function]
    pub fn extend(&mut self, value: Value) -> VmResult<()> {
        let mut it = vm_try!(value.into_iter());

        while let Some(value) = vm_try!(it.next()) {
            self.inner.push_back(value);
        }

        VmResult::Ok(())
    }

    /// Provides a reference to the front element, or `None` if the deque is
    /// empty.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::VecDeque;
    ///
    /// let d = VecDeque::new();
    /// assert_eq!(d.front(), None);
    ///
    /// d.push_back(1);
    /// d.push_back(2);
    /// assert_eq!(d.front(), Some(1));
    /// ```
    #[rune::function]
    fn front(&mut self) -> Option<Value> {
        self.inner.front().cloned()
    }

    /// Provides a reference to the back element, or `None` if the deque is
    /// empty.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::VecDeque;
    ///
    /// let d = VecDeque::new();
    /// assert_eq!(d.back(), None);
    ///
    /// d.push_back(1);
    /// d.push_back(2);
    /// assert_eq!(d.back(), Some(2));
    /// ```
    #[rune::function]
    pub fn back(&self) -> Option<Value> {
        self.inner.back().cloned()
    }

    /// Appends an element to the back of the deque.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::VecDeque;
    ///
    /// let buf = VecDeque::new();
    /// buf.push_back(1);
    /// buf.push_back(3);
    /// assert_eq!(Some(3), buf.back());
    /// ```
    #[rune::function]
    fn push_back(&mut self, value: Value) {
        self.inner.push_back(value);
    }

    /// Prepends an element to the deque.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::VecDeque;
    ///
    /// let d = VecDeque::new();
    /// d.push_front(1);
    /// d.push_front(2);
    /// assert_eq!(d.front(), Some(2));
    /// ```
    #[rune::function]
    fn push_front(&mut self, value: Value) {
        self.inner.push_front(value);
    }

    /// Removes the first element and returns it, or `None` if the deque is
    /// empty.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::VecDeque;
    ///
    /// let d = VecDeque::new();
    /// d.push_back(1);
    /// d.push_back(2);
    ///
    /// assert_eq!(d.pop_front(), Some(1));
    /// assert_eq!(d.pop_front(), Some(2));
    /// assert_eq!(d.pop_front(), None);
    /// ```
    #[rune::function]
    fn pop_front(&mut self) -> Option<Value> {
        self.inner.pop_front()
    }

    /// Removes the last element from the deque and returns it, or `None` if it
    /// is empty.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::VecDeque;
    ///
    /// let buf = VecDeque::new();
    /// assert_eq!(buf.pop_back(), None);
    /// buf.push_back(1);
    /// buf.push_back(3);
    /// assert_eq!(buf.pop_back(), Some(3));
    /// ```
    #[rune::function]
    fn pop_back(&mut self) -> Option<Value> {
        self.inner.pop_back()
    }

    /// Reserves capacity for at least `additional` more elements to be inserted
    /// in the given deque. The collection may reserve more space to
    /// speculatively avoid frequent reallocations.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity overflows `usize`.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::VecDeque;
    ///
    /// let buf = [1].into::<VecDeque>();
    /// buf.reserve(10);
    /// assert!(buf.capacity() >= 11);
    /// ```
    #[rune::function]
    fn reserve(&mut self, index: usize) {
        self.inner.reserve(index);
    }

    /// Returns the number of elements in the deque.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::VecDeque;
    ///
    /// let deque = VecDeque::new();
    /// assert_eq!(deque.len(), 0);
    /// deque.push_back(1);
    /// assert_eq!(deque.len(), 1);
    /// ```
    #[rune::function]
    fn len(&mut self) -> usize {
        self.inner.len()
    }

    /// Returns the number of elements the deque can hold without reallocating.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::VecDeque;
    ///
    /// let buf = VecDeque::with_capacity(10);
    /// assert!(buf.capacity() >= 10);
    /// ```
    #[rune::function]
    fn capacity(&mut self) -> usize {
        self.inner.capacity()
    }

    /// Inserts an element at `index` within the deque, shifting all elements
    /// with indices greater than or equal to `index` towards the back.
    ///
    /// Element at index 0 is the front of the queue.
    ///
    /// # Panics
    ///
    /// Panics if `index` is greater than deque's length
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::VecDeque;
    ///
    /// let buf = VecDeque::new();
    /// buf.push_back('a');
    /// buf.push_back('b');
    /// buf.push_back('c');
    /// assert_eq!(buf, ['a', 'b', 'c']);
    ///
    /// buf.insert(1, 'd');
    /// assert_eq!(buf, ['a', 'd', 'b', 'c']);
    /// ```
    #[rune::function]
    fn insert(&mut self, index: usize, value: Value) -> VmResult<()> {
        if index > self.inner.len() {
            return VmResult::err(VmErrorKind::OutOfRange {
                index: index.into(),
                length: self.inner.len().into(),
            });
        }

        self.inner.insert(index, value);
        VmResult::Ok(())
    }

    /// Removes and returns the element at `index` from the deque.
    /// Whichever end is closer to the removal point will be moved to make
    /// room, and all the affected elements will be moved to new positions.
    /// Returns `None` if `index` is out of bounds.
    ///
    /// Element at index 0 is the front of the queue.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::VecDeque;
    ///
    /// let buf = VecDeque::new();
    /// buf.push_back(1);
    /// buf.push_back(2);
    /// buf.push_back(3);
    /// assert_eq!(buf, [1, 2, 3]);
    ///
    /// assert_eq!(buf.remove(1), Some(2));
    /// assert_eq!(buf, [1, 3]);
    /// ```
    #[rune::function]
    fn remove(&mut self, index: usize) -> Option<Value> {
        self.inner.remove(index)
    }

    /// Rotates the double-ended queue `mid` places to the left.
    ///
    /// Equivalently,
    /// - Rotates item `mid` into the first position.
    /// - Pops the first `mid` items and pushes them to the end.
    /// - Rotates `len() - mid` places to the right.
    ///
    /// # Panics
    ///
    /// If `mid` is greater than `len()`. Note that `mid == len()` does _not_
    /// panic and is a no-op rotation.
    ///
    /// # Complexity
    ///
    /// Takes `*O*(min(mid, len() - mid))` time and no extra space.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::VecDeque;
    ///
    /// let buf = (0..10).iter().collect::<VecDeque>();
    ///
    /// buf.rotate_left(3);
    /// assert_eq!(buf, [3, 4, 5, 6, 7, 8, 9, 0, 1, 2]);
    ///
    /// for i in 1..10 {
    ///     assert_eq!(i * 3 % 10, buf[0]);
    ///     buf.rotate_left(3);
    /// }
    ///
    /// assert_eq!(buf, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    /// ```
    #[rune::function]
    fn rotate_left(&mut self, mid: usize) -> VmResult<()> {
        if mid > self.inner.len() {
            return VmResult::err(VmErrorKind::OutOfRange {
                index: mid.into(),
                length: self.inner.len().into(),
            });
        }

        self.inner.rotate_left(mid);
        VmResult::Ok(())
    }

    /// Rotates the double-ended queue `k` places to the right.
    ///
    /// Equivalently,
    /// - Rotates the first item into position `k`.
    /// - Pops the last `k` items and pushes them to the front.
    /// - Rotates `len() - k` places to the left.
    ///
    /// # Panics
    ///
    /// If `k` is greater than `len()`. Note that `k == len()` does _not_ panic
    /// and is a no-op rotation.
    ///
    /// # Complexity
    ///
    /// Takes `*O*(min(k, len() - k))` time and no extra space.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::VecDeque;
    ///
    /// let buf = (0..10).iter().collect::<VecDeque>();
    ///
    /// buf.rotate_right(3);
    /// assert_eq!(buf, [7, 8, 9, 0, 1, 2, 3, 4, 5, 6]);
    ///
    /// for i in 1..10 {
    ///     assert_eq!(0, buf[i * 3 % 10]);
    ///     buf.rotate_right(3);
    /// }
    ///
    /// assert_eq!(buf, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    /// ```
    #[rune::function]
    fn rotate_right(&mut self, mid: usize) -> VmResult<()> {
        if mid > self.inner.len() {
            return VmResult::err(VmErrorKind::OutOfRange {
                index: mid.into(),
                length: self.inner.len().into(),
            });
        }

        self.inner.rotate_right(mid);
        VmResult::Ok(())
    }

    /// Returns a front-to-back iterator.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::collections::VecDeque;
    ///
    /// let buf = VecDeque::new();
    /// buf.push_back(5);
    /// buf.push_back(3);
    /// buf.push_back(4);
    /// let b = [5, 3, 4];
    /// let c = buf;
    /// assert_eq!(c, b);
    /// ```
    #[inline]
    #[rune::function]
    fn iter(&self) -> Iterator {
        let iter = self.inner.clone().into_iter();
        Iterator::from("std::collections::vec_deque::Iter", iter)
    }

    pub(crate) fn from_iter(mut it: Iterator) -> VmResult<Self> {
        let mut inner = collections::VecDeque::with_capacity(it.size_hint().0);

        while let Some(value) = vm_try!(it.next()) {
            inner.push_back(value);
        }

        VmResult::Ok(Self { inner })
    }

    fn get(&self, index: usize) -> VmResult<Value> {
        let Some(v) = self.inner.get(index) else {
            return VmResult::err(VmErrorKind::OutOfRange {
                index: index.into(),
                length: self.inner.len().into(),
            });
        };

        VmResult::Ok(v.clone())
    }

    fn set(&mut self, index: usize, value: Value) -> VmResult<()> {
        let Some(v) = self.inner.get_mut(index) else {
            return VmResult::err(VmErrorKind::OutOfRange {
                index: index.into(),
                length: self.inner.len().into(),
            });
        };

        *v = value;
        VmResult::Ok(())
    }

    #[inline]
    fn string_debug(&self, s: &mut String) -> fmt::Result {
        write!(s, "{:?}", self.inner)
    }
}

/// Construct a [`VecDeque`] from a value.
///
/// # Examples
///
/// ```rune
/// use std::collections::VecDeque;
///
/// let buf = VecDeque::from([1, 2, 3]);
/// ```
#[rune::function(path = VecDeque::from)]
fn from(value: Value) -> VmResult<VecDeque> {
    VecDeque::from_iter(vm_try!(value.into_iter()))
}

fn eq(this: &VecDeque, other: Value) -> VmResult<bool> {
    let mut other = vm_try!(other.into_iter());

    for a in &this.inner {
        let Some(b) = vm_try!(other.next()) else {
            return VmResult::Ok(false);
        };

        if !vm_try!(Value::eq(a, &b)) {
            return VmResult::Ok(false);
        }
    }

    if vm_try!(other.next()).is_some() {
        return VmResult::Ok(false);
    }

    VmResult::Ok(true)
}
