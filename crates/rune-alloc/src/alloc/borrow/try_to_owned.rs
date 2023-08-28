use core::borrow::Borrow;

use crate::{Error, TryClone};

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
    /// use rune_alloc::{Vec, String, TryToOwned};
    ///
    /// let s: &str = "a";
    /// let ss: String = s.try_to_owned()?;
    /// # let v: &[i32] = &[1, 2];
    /// # let vv: Vec<i32> = v.try_to_owned()?;
    /// # Ok::<_, rune_alloc::Error>(())
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
