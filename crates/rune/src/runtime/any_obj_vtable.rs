use core::any::TypeId;
use core::fmt;
use core::mem::{needs_drop, offset_of, ManuallyDrop};
use core::ptr::{addr_of_mut, drop_in_place, NonNull};

use crate::alloc::alloc::Global;
use crate::alloc::{self, Box};
use crate::{Any, Hash};

use super::{AnyObj, AnyObjData, AnyTypeInfo, TypeInfo};

/// The signature of a pointer coercion function.
type TypeIdFn = fn() -> TypeId;

/// The signature of a descriptive type name function.
type DebugFn = fn(&mut fmt::Formatter<'_>) -> fmt::Result;

/// The kind of the stored value in the `AnyObj`.
enum Kind {
    /// Underlying access is shared.
    Shared,
    /// Underlying access is exclusively accessible.
    Exclusive,
    /// The underlying type is owned.
    Owned,
}

pub(super) struct AnyObjVtable {
    /// The statically known kind of reference being stored.
    kind: Kind,
    /// Punt the inner pointer to the type corresponding to the type hash.
    type_id: TypeIdFn,
    /// Static type information.
    type_info: AnyTypeInfo,
    /// Type hash of the interior type.
    type_hash: Hash,
    /// Type information for diagnostics.
    debug: DebugFn,
    /// Value drop implementation. Set to `None` if the underlying value does
    /// not need to be dropped.
    pub(super) drop_value: Option<unsafe fn(NonNull<AnyObjData>)>,
    /// Only drop the box implementation.
    pub(super) drop: unsafe fn(NonNull<AnyObjData>),
    /// Clone the literal content of the shared value.
    pub(super) clone: unsafe fn(NonNull<AnyObjData>) -> alloc::Result<AnyObj>,
}

impl AnyObjVtable {
    #[inline]
    pub(super) const fn owned<T>() -> &'static Self
    where
        T: Any,
    {
        &Self {
            kind: Kind::Owned,
            type_id: TypeId::of::<T>,
            debug: debug_ref_impl::<T>,
            type_info: T::ANY_TYPE_INFO,
            type_hash: T::HASH,
            drop_value: const {
                if needs_drop::<T>() {
                    Some(drop_value::<T>)
                } else {
                    None
                }
            },
            drop: drop_box::<ManuallyDrop<T>>,
            clone: clone_own::<T>,
        }
    }

    #[inline]
    pub(super) const fn from_ref<T>() -> &'static Self
    where
        T: Any,
    {
        &Self {
            kind: Kind::Shared,
            type_id: TypeId::of::<T>,
            debug: debug_ref_impl::<T>,
            type_info: T::ANY_TYPE_INFO,
            type_hash: T::HASH,
            drop_value: None,
            drop: drop_box::<NonNull<T>>,
            clone: clone_ref::<T>,
        }
    }

    #[inline]
    pub(super) const fn from_mut<T>() -> &'static Self
    where
        T: Any,
    {
        &Self {
            kind: Kind::Exclusive,
            type_id: TypeId::of::<T>,
            debug: debug_mut_impl::<T>,
            type_info: T::ANY_TYPE_INFO,
            type_hash: T::HASH,
            drop_value: None,
            drop: drop_box::<NonNull<T>>,
            clone: clone_mut::<T>,
        }
    }

    #[inline]
    pub(super) fn is<T>(&self) -> bool
    where
        T: 'static,
    {
        (self.type_id)() == TypeId::of::<T>()
    }

    #[inline]
    pub(super) fn type_info(&self) -> TypeInfo {
        TypeInfo::any_type_info(self.type_info)
    }

    #[inline]
    pub(super) fn type_hash(&self) -> Hash {
        self.type_hash
    }

    #[inline]
    pub(super) fn debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.debug)(f)
    }

    #[inline]
    pub(super) fn as_ptr<T>(&self, base: NonNull<AnyObjData>) -> NonNull<T> {
        if matches!(self.kind, Kind::Owned) {
            unsafe { base.byte_add(offset_of!(AnyObjData<T>, data)).cast() }
        } else {
            unsafe {
                base.byte_add(offset_of!(AnyObjData<NonNull<T>>, data))
                    .cast()
                    .read()
            }
        }
    }

    #[inline]
    pub(super) fn is_mutable(&self) -> bool {
        matches!(self.kind, Kind::Exclusive | Kind::Owned)
    }

    #[inline]
    pub(super) fn is_owned(&self) -> bool {
        matches!(self.kind, Kind::Owned)
    }
}

fn debug_ref_impl<T>(f: &mut fmt::Formatter<'_>) -> fmt::Result
where
    T: ?Sized + Any,
{
    write!(f, "&{}", T::ITEM)
}

fn debug_mut_impl<T>(f: &mut fmt::Formatter<'_>) -> fmt::Result
where
    T: ?Sized + Any,
{
    write!(f, "&mut {}", T::ITEM)
}

unsafe fn drop_value<T>(this: NonNull<AnyObjData>) {
    let data = addr_of_mut!((*this.cast::<AnyObjData<T>>().as_ptr()).data);
    drop_in_place(data);
}

unsafe fn drop_box<T>(this: NonNull<AnyObjData>) {
    let data = Box::from_raw_in(this.cast::<AnyObjData<T>>().as_ptr(), Global);
    drop(data)
}

unsafe fn clone_own<T>(this: NonNull<AnyObjData>) -> alloc::Result<AnyObj>
where
    T: Any,
{
    // NB: We read the value without deallocating it from the previous location,
    // since that would cause the returned value to be invalid.
    let value = addr_of_mut!((*this.cast::<AnyObjData<T>>().as_ptr()).data).read();
    AnyObj::new(value)
}

unsafe fn clone_ref<T>(this: NonNull<AnyObjData>) -> alloc::Result<AnyObj>
where
    T: Any,
{
    let value = addr_of_mut!((*this.cast::<AnyObjData<NonNull<T>>>().as_ptr()).data).read();
    AnyObj::from_ref(value.as_ptr().cast_const())
}

unsafe fn clone_mut<T>(this: NonNull<AnyObjData>) -> alloc::Result<AnyObj>
where
    T: Any,
{
    let value = addr_of_mut!((*this.cast::<AnyObjData<NonNull<T>>>().as_ptr()).data).read();
    AnyObj::from_mut(value.as_ptr())
}
