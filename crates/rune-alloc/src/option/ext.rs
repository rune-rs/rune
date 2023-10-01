use crate::clone::TryClone;
use crate::error::Error;

/// Extensions to `Option<T>`.
pub trait OptionExt<T> {
    /// Maps an `Option<&T>` to an `Option<T>` by cloning the contents of the
    /// option.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::prelude::*;
    ///
    /// let x = 12u32;
    /// let opt_x = Some(&x);
    /// assert_eq!(opt_x, Some(&12));
    /// let cloned = opt_x.try_cloned()?;
    /// assert_eq!(cloned, Some(12u32));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[must_use = "`self` will be dropped if the result is not used"]
    fn try_cloned(self) -> Result<Option<T>, Error>;
}

impl<T> OptionExt<T> for Option<&T>
where
    T: TryClone,
{
    fn try_cloned(self) -> Result<Option<T>, Error> {
        Ok(match self {
            Some(value) => Some(value.try_clone()?),
            None => None,
        })
    }
}
