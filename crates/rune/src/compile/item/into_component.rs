use std::hash::{self, Hash};

use smallvec::SmallVec;

use crate::compile::item::internal;
use crate::compile::{Component, ComponentRef};
use crate::runtime::RawStr;

/// Trait for encoding the current type into a [Component].
pub trait IntoComponent: Sized {
    /// Convert into a component directly.
    fn as_component_ref(&self) -> ComponentRef<'_>;

    /// Convert into component.
    #[inline]
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
    fn as_component_ref(&self) -> ComponentRef<'_> {
        *self
    }

    #[inline]
    fn into_component(self) -> Component {
        into_component(self)
    }
}

impl IntoComponent for &ComponentRef<'_> {
    fn as_component_ref(&self) -> ComponentRef<'_> {
        **self
    }

    fn into_component(self) -> Component {
        into_component(*self)
    }
}

impl IntoComponent for Component {
    fn as_component_ref(&self) -> ComponentRef<'_> {
        Component::as_component_ref(self)
    }

    fn into_component(self) -> Component {
        self
    }
}

impl IntoComponent for &Component {
    fn as_component_ref(&self) -> ComponentRef<'_> {
        Component::as_component_ref(*self)
    }

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
impl_into_component_for_str!(String, self, self.into());
impl_into_component_for_str!(&String, self, self.clone().into());
impl_into_component_for_str!(Box<str>, self, self);
impl_into_component_for_str!(&Box<str>, self, self.clone());
impl_into_component_for_str!(std::borrow::Cow<'_, str>, self, self.as_ref().into());

/// Convert into an owned component.
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
