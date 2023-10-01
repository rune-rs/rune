//! A module for working with borrowed data.

use core::borrow::Borrow;
use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::ops::Deref;

#[cfg(feature = "alloc")]
use ::rust_alloc::borrow::ToOwned;

use crate::clone::TryClone;
use crate::error::Error;
use crate::vec::Vec;

/// A generalization of `TryClone` to borrowed data.
///
/// Some types make it possible to go from borrowed to owned, usually by
/// implementing the `TryClone` trait. But `TryClone` works only for going from
/// `&T` to `T`. The `ToOwned` trait generalizes `TryClone` to construct owned
/// data from any borrow of a given type.
pub trait TryToOwned {
    /// The resulting type after obtaining ownership.
    type Owned: Borrow<Self>;

    /// Creates owned data from borrowed data, usually by cloning.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::{Vec, String};
    /// use rune::alloc::prelude::*;
    ///
    /// let s: &str = "a";
    /// let ss: String = s.try_to_owned()?;
    /// # let v: &[i32] = &[1, 2];
    /// # let vv: Vec<i32> = v.try_to_owned()?;
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    fn try_to_owned(&self) -> Result<Self::Owned, Error>;
}

impl<T> TryToOwned for T
where
    T: TryClone,
{
    type Owned = T;

    #[inline]
    fn try_to_owned(&self) -> Result<T, Error> {
        self.try_clone()
    }
}

impl TryToOwned for crate::path::Path {
    type Owned = crate::path::PathBuf;

    fn try_to_owned(&self) -> Result<Self::Owned, Error> {
        Ok(self.to_path_buf())
    }
}

/// A clone-on-write smart pointer.
///
/// The type `Cow` is a smart pointer providing clone-on-write functionality: it
/// can enclose and provide immutable access to borrowed data, and clone the
/// data lazily when mutation or ownership is required. The type is designed to
/// work with general borrowed data via the `Borrow` trait.
///
/// `Cow` implements `Deref`, which means that you can call non-mutating methods
/// directly on the data it encloses. If mutation is desired, `to_mut` will
/// obtain a mutable reference to an owned value, cloning if necessary.
///
/// If you need reference-counting pointers, note that
/// [`Rc::make_mut`][rust_alloc::rc::Rc::make_mut] and
/// [`Arc::make_mut`][rust_alloc::sync::Arc::make_mut] can provide
/// clone-on-write functionality as well.
///
/// # Examples
///
/// ```
/// use rune::alloc::borrow::Cow;
/// use rune::alloc::try_vec;
/// use rune::alloc::prelude::*;
///
/// fn abs_all(input: &mut Cow<'_, [i32]>) -> rune::alloc::Result<()> {
///     for i in 0..input.len() {
///         let v = input[i];
///         if v < 0 {
///             // Clones into a vector if not already owned.
///             input.try_to_mut()?[i] = -v;
///         }
///     }
///
///     Ok(())
/// }
///
/// // No clone occurs because `input` doesn't need to be mutated.
/// let slice = [0, 1, 2];
/// let mut input = Cow::from(&slice[..]);
/// abs_all(&mut input)?;
///
/// // Clone occurs because `input` needs to be mutated.
/// let slice = [-1, 0, 1];
/// let mut input = Cow::from(&slice[..]);
/// abs_all(&mut input)?;
///
/// // No clone occurs because `input` is already owned.
/// let mut input = Cow::from(try_vec![-1, 0, 1]);
/// abs_all(&mut input)?;
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// Another example showing how to keep `Cow` in a struct:
///
/// ```
/// use rune::alloc::Vec;
/// use rune::alloc::borrow::Cow;
/// use rune::alloc::prelude::*;
///
/// struct Items<'a, X> where [X]: TryToOwned<Owned = Vec<X>> {
///     values: Cow<'a, [X]>,
/// }
///
/// impl<'a, X: TryClone + 'a> Items<'a, X> where [X]: TryToOwned<Owned = Vec<X>> {
///     fn new(v: Cow<'a, [X]>) -> Self {
///         Items { values: v }
///     }
/// }
///
/// // Creates a container from borrowed values of a slice
/// let readonly = [1, 2];
/// let borrowed = Items::new((&readonly[..]).into());
/// match borrowed {
///     Items { values: Cow::Borrowed(b) } => println!("borrowed {b:?}"),
///     _ => panic!("expect borrowed value"),
/// }
///
/// let mut clone_on_write = borrowed;
/// // Mutates the data from slice into owned vec and pushes a new value on top
/// clone_on_write.values.try_to_mut()?.try_push(3)?;
/// println!("clone_on_write = {:?}", clone_on_write.values);
///
/// // The data was mutated. Let's check it out.
/// match clone_on_write {
///     Items { values: Cow::Owned(_) } => println!("clone_on_write contains owned data"),
///     _ => panic!("expect owned data"),
/// }
/// # Ok::<_, rune::alloc::Error>(())
/// ```
pub enum Cow<'b, T: ?Sized + 'b>
where
    T: TryToOwned,
{
    /// Borrowed data.
    Borrowed(&'b T),
    /// Owned data.
    Owned(<T as TryToOwned>::Owned),
}

impl<B: ?Sized + TryToOwned> Cow<'_, B> {
    /// Returns true if the data is borrowed, i.e. if `to_mut` would require
    /// additional work.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::borrow::Cow;
    /// use rune::alloc::prelude::*;
    ///
    /// let cow = Cow::Borrowed("moo");
    /// assert!(cow.is_borrowed());
    ///
    /// let bull: Cow<'_, str> = Cow::Owned("...moo?".try_to_string()?);
    /// assert!(!bull.is_borrowed());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub const fn is_borrowed(&self) -> bool {
        matches!(self, Cow::Borrowed(..))
    }

    /// Returns true if the data is owned, i.e. if `to_mut` would be a no-op.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::borrow::Cow;
    /// use rune::alloc::prelude::*;
    ///
    /// let cow: Cow<'_, str> = Cow::Owned("moo".try_to_string()?);
    /// assert!(cow.is_owned());
    ///
    /// let bull = Cow::Borrowed("...moo?");
    /// assert!(!bull.is_owned());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub const fn is_owned(&self) -> bool {
        !self.is_borrowed()
    }

    /// Acquires a mutable reference to the owned form of the data.
    ///
    /// Clones the data if it is not already owned.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::borrow::Cow;
    /// use rune::alloc::String;
    ///
    /// let mut cow = Cow::Borrowed("foo");
    /// cow.try_to_mut()?.make_ascii_uppercase();
    ///
    /// assert_eq!(cow, Cow::Owned(String::try_from("FOO")?));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn try_to_mut(&mut self) -> Result<&mut <B as TryToOwned>::Owned, Error> {
        Ok(match *self {
            Cow::Borrowed(borrowed) => {
                *self = Cow::Owned(borrowed.try_to_owned()?);

                match *self {
                    Cow::Borrowed(..) => unreachable!(),
                    Cow::Owned(ref mut owned) => owned,
                }
            }
            Cow::Owned(ref mut owned) => owned,
        })
    }

    /// Extracts the owned data.
    ///
    /// Clones the data if it is not already owned.
    ///
    /// # Examples
    ///
    /// Calling `into_owned` on a `Cow::Borrowed` returns a clone of the borrowed data:
    ///
    /// ```
    /// use rune::alloc::borrow::Cow;
    /// use rune::alloc::String;
    ///
    /// let s = "Hello world!";
    /// let cow = Cow::Borrowed(s);
    ///
    /// assert_eq!(cow.try_into_owned()?, String::try_from(s)?);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    ///
    /// Calling `into_owned` on a `Cow::Owned` returns the owned data. The data is moved out of the
    /// `Cow` without being cloned.
    ///
    /// ```
    /// use rune::alloc::borrow::Cow;
    /// use rune::alloc::String;
    ///
    /// let s = "Hello world!";
    /// let cow: Cow<'_, str> = Cow::Owned(String::try_from(s)?);
    ///
    /// assert_eq!(cow.try_into_owned()?, String::try_from(s)?);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn try_into_owned(self) -> Result<<B as TryToOwned>::Owned, Error> {
        match self {
            Cow::Borrowed(borrowed) => borrowed.try_to_owned(),
            Cow::Owned(owned) => Ok(owned),
        }
    }
}

impl<'a, T: ?Sized + 'a> From<&'a T> for Cow<'a, T>
where
    T: TryToOwned,
{
    /// Construct a `Cow` from a reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::borrow::Cow;
    ///
    /// let s = Cow::from("Hello World");
    /// assert_eq!("Hello World", s);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    fn from(b: &'a T) -> Self {
        Cow::Borrowed(b)
    }
}

#[cfg(feature = "alloc")]
impl<'a, T: ?Sized + 'a> TryFrom<rust_alloc::borrow::Cow<'a, T>> for Cow<'a, T>
where
    T: ToOwned + TryToOwned,
    <T as TryToOwned>::Owned: TryFrom<<T as ToOwned>::Owned>,
{
    type Error = <<T as TryToOwned>::Owned as TryFrom<<T as ToOwned>::Owned>>::Error;

    fn try_from(value: rust_alloc::borrow::Cow<'a, T>) -> Result<Self, Self::Error> {
        Ok(match value {
            rust_alloc::borrow::Cow::Borrowed(b) => Cow::Borrowed(b),
            rust_alloc::borrow::Cow::Owned(o) => Cow::Owned(<T as TryToOwned>::Owned::try_from(o)?),
        })
    }
}

impl<B: ?Sized + TryToOwned> Deref for Cow<'_, B>
where
    B::Owned: Borrow<B>,
{
    type Target = B;

    fn deref(&self) -> &B {
        match *self {
            Cow::Borrowed(borrowed) => borrowed,
            Cow::Owned(ref owned) => owned.borrow(),
        }
    }
}

impl<T: ?Sized + TryToOwned> AsRef<T> for Cow<'_, T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self
    }
}

impl<T: ?Sized> fmt::Display for Cow<'_, T>
where
    T: fmt::Display + TryToOwned,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: ?Sized> fmt::Debug for Cow<'_, T>
where
    T: fmt::Debug + TryToOwned,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl TryClone for Cow<'_, str> {
    #[inline]
    fn try_clone(&self) -> Result<Self, Error> {
        Ok(match self {
            Cow::Borrowed(b) => Cow::Borrowed(b),
            Cow::Owned(o) => Cow::Owned(o.try_clone()?),
        })
    }
}

impl<T> From<Vec<T>> for Cow<'_, [T]>
where
    T: TryClone,
{
    fn from(vec: Vec<T>) -> Self {
        Cow::Owned(vec)
    }
}

impl<B: ?Sized> PartialEq for Cow<'_, B>
where
    B: PartialEq + TryToOwned,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        (**self).eq(&**other)
    }
}

impl<B: ?Sized> Eq for Cow<'_, B> where B: Eq + TryToOwned {}

impl<B: ?Sized> PartialOrd for Cow<'_, B>
where
    B: PartialOrd + TryToOwned,
{
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        (**self).partial_cmp(&**other)
    }
}

impl<B: ?Sized> Ord for Cow<'_, B>
where
    B: Ord + TryToOwned,
{
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        (**self).cmp(&**other)
    }
}

impl<B: ?Sized> Hash for Cow<'_, B>
where
    B: Hash + TryToOwned,
{
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&**self, state)
    }
}
