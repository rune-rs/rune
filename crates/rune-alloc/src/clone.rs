//! The `TryClone` trait for types that cannot be 'implicitly copied'.
//!
//! In Rust, some simple types are "implicitly copyable" and when you assign
//! them or pass them as arguments, the receiver will get a copy, leaving the
//! original value in place. These types do not require allocation to copy and
//! do not have finalizers (i.e., they do not contain owned boxes or implement
//! [`Drop`]), so the compiler considers them cheap and safe to copy. For other
//! types copies must be made explicitly, by convention implementing the
//! [`TryClone`] trait and calling the [`try_clone`] method.
//!
//! [`try_clone`]: TryClone::try_clone
//!
//! Basic usage example:
//!
//! ```
//! use rune::alloc::String;
//! use rune::alloc::prelude::*;
//!
//! // String type implements TryClone
//! let s = String::new();
//! // ... so we can clone it
//! let copy = s.try_clone()?;
//! # Ok::<_, rune::alloc::Error>(())
//! ```
//!
//! To easily implement the TryClone trait, you can also use
//! `#[derive(TryClone)]`. Example:
//!
//! ```
//! use rune::alloc::prelude::*;
//!
//! // we add the TryClone trait to Morpheus struct
//! #[derive(TryClone)]
//! struct Morpheus {
//!    blue_pill: f32,
//!    red_pill: i64,
//! }
//!
//! let f = Morpheus { blue_pill: 0.0, red_pill: 0 };
//! // and now we can clone it!
//! let copy = f.try_clone()?;
//! # Ok::<_, rune::alloc::Error>(())
//! ```

use crate::error::Error;

#[doc(inline)]
pub use rune_alloc_macros::TryClone;

/// Fallible `TryClone` trait.
pub trait TryClone: Sized {
    /// Try to clone the current value, raising an allocation error if it's unsuccessful.
    fn try_clone(&self) -> Result<Self, Error>;

    /// Performs copy-assignment from `source`.
    ///
    /// `a.try_clone_from(&b)` is equivalent to `a = b.clone()` in
    /// functionality, but can be overridden to reuse the resources of `a` to
    /// avoid unnecessary allocations.
    #[inline]
    fn try_clone_from(&mut self, source: &Self) -> Result<(), Error> {
        *self = source.try_clone()?;
        Ok(())
    }
}

/// Marker trait for types which are `Copy`.
#[cfg_attr(rune_nightly, rustc_specialization_trait)]
pub trait TryCopy: TryClone {}

impl<T: ?Sized> TryClone for &T {
    fn try_clone(&self) -> Result<Self, Error> {
        Ok(*self)
    }
}

macro_rules! impl_tuple {
    ($count:expr $(, $ty:ident $var:ident $num:expr)*) => {
        impl<$($ty,)*> TryClone for ($($ty,)*) where $($ty: TryClone,)* {
            #[inline]
            fn try_clone(&self) -> Result<Self, Error> {
                let ($($var,)*) = self;
                Ok(($($var.try_clone()?,)*))
            }
        }
    }
}

repeat_macro!(impl_tuple);

macro_rules! impl_copy {
    ($ty:ty) => {
        impl TryClone for $ty {
            #[inline]
            fn try_clone(&self) -> Result<Self, Error> {
                Ok(*self)
            }
        }

        impl TryCopy for $ty {}
    };
}

impl_copy!(char);
impl_copy!(bool);
impl_copy!(usize);
impl_copy!(isize);
impl_copy!(u8);
impl_copy!(u16);
impl_copy!(u32);
impl_copy!(u64);
impl_copy!(u128);
impl_copy!(i8);
impl_copy!(i16);
impl_copy!(i32);
impl_copy!(i64);
impl_copy!(i128);
impl_copy!(f32);
impl_copy!(f64);

impl_copy!(::core::num::NonZeroUsize);
impl_copy!(::core::num::NonZeroIsize);
impl_copy!(::core::num::NonZeroU8);
impl_copy!(::core::num::NonZeroU16);
impl_copy!(::core::num::NonZeroU32);
impl_copy!(::core::num::NonZeroU64);
impl_copy!(::core::num::NonZeroU128);
impl_copy!(::core::num::NonZeroI8);
impl_copy!(::core::num::NonZeroI16);
impl_copy!(::core::num::NonZeroI32);
impl_copy!(::core::num::NonZeroI64);
impl_copy!(::core::num::NonZeroI128);

impl<T, E> TryClone for ::core::result::Result<T, E>
where
    T: TryClone,
    E: TryClone,
{
    #[inline]
    fn try_clone(&self) -> Result<Self, Error> {
        Ok(match self {
            Ok(value) => Ok(value.try_clone()?),
            Err(value) => Err(value.try_clone()?),
        })
    }
}

impl<T> TryClone for ::core::option::Option<T>
where
    T: TryClone,
{
    #[inline]
    fn try_clone(&self) -> Result<Self, Error> {
        Ok(match self {
            Some(value) => Some(value.try_clone()?),
            None => None,
        })
    }
}

#[cfg(feature = "alloc")]
impl<T: ?Sized> TryClone for ::rust_alloc::sync::Arc<T> {
    fn try_clone(&self) -> Result<Self, Error> {
        Ok(self.clone())
    }
}

#[cfg(feature = "alloc")]
impl<T: ?Sized> TryClone for ::rust_alloc::rc::Rc<T> {
    fn try_clone(&self) -> Result<Self, Error> {
        Ok(self.clone())
    }
}

#[cfg(feature = "alloc")]
impl<T> TryClone for ::rust_alloc::boxed::Box<T>
where
    T: TryClone,
{
    fn try_clone(&self) -> Result<Self, Error> {
        Ok(::rust_alloc::boxed::Box::new(self.as_ref().try_clone()?))
    }
}

#[cfg(feature = "alloc")]
impl<T> TryClone for ::rust_alloc::boxed::Box<[T]>
where
    T: TryClone,
{
    fn try_clone(&self) -> Result<Self, Error> {
        // TODO: use a fallible box allocation.
        let mut out = ::rust_alloc::vec::Vec::with_capacity(self.len());

        for value in self.iter() {
            out.push(value.try_clone()?);
        }

        Ok(out.into())
    }
}

#[cfg(feature = "alloc")]
impl TryClone for ::rust_alloc::string::String {
    #[inline]
    fn try_clone(&self) -> Result<Self, Error> {
        // TODO: use fallible allocations for component.
        Ok(self.clone())
    }
}

#[cfg(all(test, feature = "alloc"))]
impl<T> TryClone for ::rust_alloc::vec::Vec<T>
where
    T: TryClone,
{
    #[inline]
    fn try_clone(&self) -> Result<Self, Error> {
        let mut out = ::rust_alloc::vec::Vec::with_capacity(self.len());

        for value in self {
            out.push(value.try_clone()?);
        }

        Ok(out)
    }
}

impl TryClone for crate::path::PathBuf {
    fn try_clone(&self) -> Result<Self, Error> {
        Ok(self.clone())
    }
}
