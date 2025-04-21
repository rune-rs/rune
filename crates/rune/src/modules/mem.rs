//! Working with memory.

use crate as rune;
use crate::alloc::fmt::TryWrite;
use crate::runtime::{self, Formatter, Value, VmResult};
use crate::{Any, ContextError, Module};

/// Working with memory.
#[rune::module(::std::mem)]
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::from_meta(self::module__meta)?;
    m.function_meta(drop)?;
    m.function_meta(snapshot)?;

    m.ty::<Snapshot>()?;
    m.function_meta(Snapshot::display)?;
    m.function_meta(Snapshot::debug)?;
    m.function_meta(Snapshot::shared)?;
    m.function_meta(Snapshot::is_exclusive)?;
    m.function_meta(Snapshot::is_readable)?;
    m.function_meta(Snapshot::is_writable)?;

    Ok(m)
}

#[derive(Any)]
#[rune(item = ::std::mem)]
struct Snapshot {
    inner: runtime::Snapshot,
}

impl Snapshot {
    /// The number of shared references to the value.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::mem::snapshot;
    ///
    /// let v = [1, 2, 3];
    ///
    /// let s = snapshot(v)?;
    /// assert_eq!(s.shared(), 0);
    ///
    /// // An iterators takes a shared reference to the collection being iterated over.
    /// let it = v.iter();
    ///
    /// let s = snapshot(v)?;
    /// assert_eq!(s.shared(), 1);
    /// drop(it);
    ///
    /// let s = snapshot(v)?;
    /// assert_eq!(s.shared(), 0);
    /// ```
    #[rune::function]
    fn shared(&self) -> usize {
        self.inner.shared()
    }

    /// Test if the snapshot indicates that the value is exclusively held.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::mem::snapshot;
    ///
    /// let v = [1, 2, 3];
    ///
    /// let s = snapshot(v)?;
    /// assert_eq!(s.shared(), 0);
    /// assert!(!s.is_exclusive());
    /// assert!(s.is_readable());
    /// assert!(s.is_writable());
    ///
    /// // Assign to a separate variable since the compiler will notice that `v` is moved.
    /// let u = v;
    ///
    /// // Move the value into a closure, causing the original reference to become exclusively held.
    /// let closure = move || {
    ///    v
    /// };
    ///
    /// let s = snapshot(u)?;
    /// assert!(s.is_exclusive());
    /// assert!(!s.is_readable());
    /// assert!(!s.is_writable());
    /// ```
    #[rune::function]
    fn is_exclusive(&self) -> bool {
        self.inner.is_exclusive()
    }

    /// Test if the snapshot indicates that the value is readable.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::mem::snapshot;
    ///
    /// let v = [1, 2, 3];
    ///
    /// let s = snapshot(v)?;
    /// assert_eq!(s.shared(), 0);
    /// assert!(!s.is_exclusive());
    /// assert!(s.is_readable());
    /// assert!(s.is_writable());
    ///
    /// // Assign to a separate variable since the compiler will notice that `v` is moved.
    /// let u = v;
    ///
    /// // Move the value into a closure, causing the original reference to become exclusively held.
    /// let closure = move || {
    ///    v
    /// };
    ///
    /// let s = snapshot(u)?;
    /// assert!(s.is_exclusive());
    /// assert!(!s.is_readable());
    /// assert!(!s.is_writable());
    /// ```
    #[rune::function]
    fn is_readable(&self) -> bool {
        self.inner.is_readable()
    }

    /// Test if the snapshot indicates that the value is writable.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::mem::snapshot;
    ///
    /// let v = [1, 2, 3];
    ///
    /// let s = snapshot(v)?;
    /// assert_eq!(s.shared(), 0);
    /// assert!(!s.is_exclusive());
    /// assert!(s.is_readable());
    /// assert!(s.is_writable());
    ///
    /// // Assign to a separate variable since the compiler will notice that `v` is moved.
    /// let u = v;
    ///
    /// // Move the value into a closure, causing the original reference to become exclusively held.
    /// let closure = move || {
    ///    v
    /// };
    ///
    /// let s = snapshot(u)?;
    /// assert!(s.is_exclusive());
    /// assert!(!s.is_readable());
    /// assert!(!s.is_writable());
    /// ```
    #[rune::function]
    fn is_writable(&self) -> bool {
        self.inner.is_writable()
    }

    #[rune::function(protocol = DISPLAY_FMT)]
    fn display(&self, f: &mut Formatter) -> VmResult<()> {
        vm_write!(f, "{}", self.inner)
    }

    #[rune::function(protocol = DEBUG_FMT)]
    fn debug(&self, f: &mut Formatter) -> VmResult<()> {
        vm_write!(f, "{:?}", self.inner)
    }
}

/// Explicitly drop the given value, freeing up any memory associated with it.
///
/// Normally values are dropped as they go out of scope, but with this method it
/// can be explicitly controlled instead.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let v = [1, 2, 3];
/// drop(v);
/// ```
#[rune::function]
fn drop(value: Value) -> VmResult<()> {
    vm_try!(value.drop());
    VmResult::Ok(())
}

/// Get the usage snapshot of a value.
///
/// A snapshot can be used to diagnose how many users a given value has.
///
/// # Examples
///
/// ```rune
/// use std::mem::snapshot;
///
/// let v1 = [1, 2, 3];
/// let a = snapshot(v1)?;
/// let v2 = [v1];
/// let b = snapshot(v1)?;
///
/// assert_eq!(a.shared(), 0);
/// assert_eq!(b.shared(), 0);
///
/// dbg!(a);
/// dbg!(b);
/// ```
#[rune::function]
fn snapshot(value: Value) -> Option<Snapshot> {
    value
        .snapshot()
        .map(|snapshot| Snapshot { inner: snapshot })
}
