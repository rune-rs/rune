//! Types relates to working with slices.

use crate as rune;
use crate::runtime::slice::Iter;
use crate::{ContextError, Module};

/// Types related to working with contiguous slices.
///
/// # Examples
///
/// ```rune
/// let it = [10, 20].iter();
///
/// assert_eq!(it.next(), Some(10));
/// assert_eq!(it.next(), Some(20));
/// assert_eq!(it.next(), None);
/// ```
#[rune::module(::std::slice)]
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::from_meta(self::module_meta)?;

    m.ty::<Iter>()?;
    m.function_meta(Iter::next__meta)?;
    m.function_meta(Iter::size_hint__meta)?;
    m.function_meta(Iter::len__meta)?;
    m.function_meta(Iter::nth__meta)?;
    m.function_meta(Iter::next_back__meta)?;
    m.implement_trait::<Iter>(rune::item!(::std::iter::Iterator))?;
    m.implement_trait::<Iter>(rune::item!(::std::iter::DoubleEndedIterator))?;

    Ok(m)
}
