use core::hash::{self, Hash};

#[cfg(feature = "alloc")]
use crate::alloc::alloc::Allocator;
#[cfg(feature = "alloc")]
use crate::alloc::borrow::Cow;
#[cfg(feature = "alloc")]
use crate::alloc::clone::TryClone;
#[cfg(feature = "alloc")]
use crate::alloc::{self, Box, String, Vec};

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
    fn into_component(self) -> alloc::Result<Component> {
        into_component(self.as_component_ref())
    }

    /// Write a component directly to a buffer.
    #[inline]
    #[doc(hidden)]
    #[cfg(feature = "alloc")]
    fn write_component<A: Allocator>(self, output: &mut Vec<u8, A>) -> alloc::Result<()> {
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

/// IntoCompoment implementation preserved for backwards compatibility.
impl<T> IntoComponent for [T; 1]
where
    T: IntoComponent,
{
    fn as_component_ref(&self) -> ComponentRef<'_> {
        let [this] = self;
        this.as_component_ref()
    }

    #[inline]
    #[cfg(feature = "alloc")]
    fn into_component(self) -> alloc::Result<Component> {
        let [this] = self;
        this.into_component()
    }

    #[inline]
    #[doc(hidden)]
    #[cfg(feature = "alloc")]
    fn write_component<A: Allocator>(self, output: &mut Vec<u8, A>) -> alloc::Result<()> {
        let [this] = self;
        this.write_component(output)
    }

    #[inline]
    fn hash_component<H>(self, hasher: &mut H)
    where
        H: hash::Hasher,
    {
        let [this] = self;
        this.hash_component(hasher)
    }
}

impl IntoComponent for ComponentRef<'_> {
    #[inline]
    fn as_component_ref(&self) -> ComponentRef<'_> {
        *self
    }

    #[inline]
    #[cfg(feature = "alloc")]
    fn into_component(self) -> alloc::Result<Component> {
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
    fn into_component(self) -> alloc::Result<Component> {
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
    fn into_component(self) -> alloc::Result<Component> {
        Ok(self)
    }
}

#[cfg(feature = "alloc")]
impl IntoComponent for &Component {
    #[inline]
    fn as_component_ref(&self) -> ComponentRef<'_> {
        Component::as_component_ref(self)
    }

    #[inline]
    fn into_component(self) -> alloc::Result<Component> {
        self.try_clone()
    }
}

macro_rules! impl_into_component_for_str {
    ($ty:ty, $slf:ident, $into:expr) => {
        impl IntoComponent for $ty {
            fn as_component_ref(&self) -> ComponentRef<'_> {
                ComponentRef::Str(self.as_ref())
            }

            #[cfg(feature = "alloc")]
            fn into_component($slf) -> alloc::Result<Component> {
                Ok(Component::Str($into))
            }

            #[cfg(feature = "alloc")]
            fn write_component<A: Allocator>(self, output: &mut Vec<u8, A>) -> alloc::Result<()> {
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

impl_into_component_for_str!(&str, self, self.try_into()?);
impl_into_component_for_str!(&&str, self, (*self).try_into()?);
impl_into_component_for_str!(RawStr, self, (*self).try_into()?);
impl_into_component_for_str!(&RawStr, self, (**self).try_into()?);
#[cfg(feature = "alloc")]
impl_into_component_for_str!(String, self, self.as_str().try_into()?);
#[cfg(feature = "alloc")]
impl_into_component_for_str!(&String, self, self.as_str().try_into()?);
#[cfg(feature = "alloc")]
impl_into_component_for_str!(Box<str>, self, self);
#[cfg(feature = "alloc")]
impl_into_component_for_str!(&Box<str>, self, self.try_clone()?);
#[cfg(feature = "alloc")]
impl_into_component_for_str!(Cow<'_, str>, self, self.as_ref().try_into()?);
#[cfg(feature = "alloc")]
impl_into_component_for_str!(
    ::rust_alloc::borrow::Cow<'_, str>,
    self,
    self.as_ref().try_into()?
);

/// Convert into an owned component.
#[cfg(feature = "alloc")]
fn into_component(component: ComponentRef<'_>) -> alloc::Result<Component> {
    Ok(match component {
        ComponentRef::Crate(s) => Component::Crate(s.try_into()?),
        ComponentRef::Str(s) => Component::Str(s.try_into()?),
        ComponentRef::Id(n) => Component::Id(n),
    })
}

/// Write the current component to the given vector.
#[cfg(feature = "alloc")]
fn write_component<A: Allocator>(
    component: ComponentRef<'_>,
    output: &mut Vec<u8, A>,
) -> alloc::Result<()> {
    match component {
        ComponentRef::Crate(s) => internal::write_crate(s, output),
        ComponentRef::Str(s) => internal::write_str(s, output),
        ComponentRef::Id(c) => internal::write_tag(output, internal::ID, c),
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
