use core::hash::{self, Hash};

#[cfg(feature = "alloc")]
use alloc::borrow::Cow;
#[cfg(feature = "alloc")]
use alloc::boxed::Box;
#[cfg(feature = "alloc")]
use alloc::string::String;

use smallvec::SmallVec;

#[cfg(feature = "alloc")]
use crate::item::Component;
use crate::item::{internal, ComponentRef};
use crate::raw_str::RawStr;

/// Trait for encoding the current type into a [Component].
pub trait IntoComponent: Sized {
    /// Convert into a component directly.
    fn as_component_ref(&self) -> ComponentRef<'_>;

    /// Convert into component.
    #[inline]
    #[cfg(feature = "alloc")]
    fn into_component(self) -> Component {
        into_component(self.as_component_ref())
    }

    /// Write a component directly to a buffer.
    #[inline]
    #[doc(hidden)]
    fn write_component(self, output: &mut SmallVec<[u8; internal::INLINE]>) {
        write_component(self.as_component_ref(), output)
    }

    /// Hash the current component.
    #[inline]
    #[doc(hidden)]
    fn hash_component<H>(self, hasher: &mut H)
    where
        H: hash::Hasher,
    {
        hash_component(self.as_component_ref(), hasher)
    }
}

impl IntoComponent for ComponentRef<'_> {
    #[inline]
    fn as_component_ref(&self) -> ComponentRef<'_> {
        *self
    }

    #[inline]
    #[cfg(feature = "alloc")]
    fn into_component(self) -> Component {
        into_component(self)
    }
}

impl IntoComponent for &ComponentRef<'_> {
    #[inline]
    fn as_component_ref(&self) -> ComponentRef<'_> {
        **self
    }

    #[inline]
    #[cfg(feature = "alloc")]
    fn into_component(self) -> Component {
        into_component(*self)
    }
}

#[cfg(feature = "alloc")]
impl IntoComponent for Component {
    #[inline]
    fn as_component_ref(&self) -> ComponentRef<'_> {
        Component::as_component_ref(self)
    }

    #[inline]
    fn into_component(self) -> Component {
        self
    }
}

#[cfg(feature = "alloc")]
impl IntoComponent for &Component {
    #[inline]
    fn as_component_ref(&self) -> ComponentRef<'_> {
        Component::as_component_ref(self)
    }

    #[inline]
    fn into_component(self) -> Component {
        self.clone()
    }
}

macro_rules! impl_into_component_for_str {
    ($ty:ty, $slf:ident, $into:expr) => {
        impl IntoComponent for $ty {
            fn as_component_ref(&self) -> ComponentRef<'_> {
                ComponentRef::Str(self.as_ref())
            }

            #[cfg(feature = "alloc")]
            fn into_component($slf) -> Component {
                Component::Str($into)
            }

            fn write_component(self, output: &mut smallvec::SmallVec<[u8; internal::INLINE]>) {
                internal::write_str(self.as_ref(), output)
            }

            fn hash_component<H>(self, hasher: &mut H)
            where
                H: hash::Hasher,
            {
                internal::hash_str(self.as_ref(), hasher);
            }
        }
    }
}

impl_into_component_for_str!(&str, self, self.into());
impl_into_component_for_str!(&&str, self, (*self).into());
impl_into_component_for_str!(RawStr, self, (*self).into());
impl_into_component_for_str!(&RawStr, self, (**self).into());
#[cfg(feature = "alloc")]
impl_into_component_for_str!(String, self, self.into());
#[cfg(feature = "alloc")]
impl_into_component_for_str!(&String, self, self.clone().into());
#[cfg(feature = "alloc")]
impl_into_component_for_str!(Box<str>, self, self);
#[cfg(feature = "alloc")]
impl_into_component_for_str!(&Box<str>, self, self.clone());
#[cfg(feature = "alloc")]
impl_into_component_for_str!(Cow<'_, str>, self, self.as_ref().into());

/// Convert into an owned component.
#[cfg(feature = "alloc")]
fn into_component(component: ComponentRef<'_>) -> Component {
    match component {
        ComponentRef::Crate(s) => Component::Crate(s.into()),
        ComponentRef::Str(s) => Component::Str(s.into()),
        ComponentRef::Id(n) => Component::Id(n),
    }
}

/// Write the current component to the given vector.
fn write_component(component: ComponentRef<'_>, output: &mut SmallVec<[u8; internal::INLINE]>) {
    match component {
        ComponentRef::Crate(s) => {
            internal::write_crate(s, output);
        }
        ComponentRef::Str(s) => {
            internal::write_str(s, output);
        }
        ComponentRef::Id(c) => {
            internal::write_tag(output, internal::ID, c);
        }
    }
}

/// Hash the current component to the given hasher.
fn hash_component<H>(component: ComponentRef<'_>, hasher: &mut H)
where
    H: hash::Hasher,
{
    match component {
        ComponentRef::Crate(s) => {
            internal::CRATE.hash(hasher);
            s.hash(hasher);
        }
        ComponentRef::Str(s) => {
            internal::STRING.hash(hasher);
            s.hash(hasher);
        }
        ComponentRef::Id(c) => {
            internal::ID.hash(hasher);
            c.hash(hasher);
        }
    }
}
