//! Storage for static items declared in a unit.

use core::cell::RefCell;
use core::fmt;

use rust_alloc::rc::Rc;

use crate::alloc::clone::TryClone;
use crate::alloc::{self, Box, Vec};
use crate::hash::ToTypeHash;
use crate::runtime::{Unit, Value};
use crate::sync::Arc;
use crate::{Hash, ItemBuf};

/// Storage for the static items declared by a [`Unit`].
///
/// A unit assigns every `static` item it declares a zero-indexed slot. The
/// values living in those slots are *not* stored in the unit itself, they are
/// stored here and handed to a virtual machine when it is called. This means a
/// single compiled unit can back any number of independent states.
///
/// Every slot keeps track of whether it has been initialized. A slot which has
/// an initializer is lazily initialized the first time a script reads it, but a
/// caller can also assign a value up front, in which case the initializer is
/// never evaluated.
///
/// The storage is a cheaply cloned handle. Cloning it does not copy the slots,
/// it produces a second handle to the same storage, which is how a caller can
/// keep reading and writing static items while a virtual machine using them is
/// running.
///
/// # Examples
///
/// ```
/// use rune::{Context, Diagnostics, Source, Sources, Vm};
/// use rune::runtime::Globals;
/// use rune::sync::Arc;
///
/// let context = Context::with_default_modules()?;
/// let runtime = Arc::try_new(context.runtime()?)?;
///
/// let mut sources = Sources::new();
/// sources.insert(Source::memory(r#"
/// static COUNTER = 0;
///
/// pub fn main() {
///     COUNTER += 1;
/// }
/// "#)?)?;
///
/// let mut diagnostics = Diagnostics::new();
///
/// let unit = rune::prepare(&mut sources)
///     .with_context(&context)
///     .with_diagnostics(&mut diagnostics)
///     .build()?;
///
/// let unit = Arc::try_new(unit)?;
/// let globals = Globals::new(unit.clone())?;
///
/// let mut vm = Vm::new(runtime, unit).with_globals(globals.clone());
/// vm.call(["main"], ())?;
/// vm.call(["main"], ())?;
///
/// let counter = globals.get(["COUNTER"])?.expect("COUNTER to be initialized");
/// assert_eq!(rune::from_value::<i64>(counter)?, 2);
/// # Ok::<_, rune::support::Error>(())
/// ```
#[derive(Default, Clone)]
pub struct Globals {
    /// `None` means that no storage has been configured. This exists so that
    /// [`Globals::empty`] can be a `const fn`, which in turn keeps [`Vm::new`]
    /// allocation free.
    ///
    /// The handle is non-atomically counted, since the storage holds [`Value`]s
    /// and can therefore never be shared across threads anyway.
    ///
    /// [`Vm::new`]: crate::runtime::Vm::new
    inner: Option<Rc<GlobalsInner>>,
}

/// The shared allocation backing a [`Globals`] handle.
///
/// This is only public to the crate so that the thread-local environment can
/// store it as a raw pointer, the same way it stores the unit and the context.
pub(crate) struct GlobalsInner {
    unit: Arc<Unit>,
    /// One slot per static declared by the unit. The number of statics is fixed
    /// by the unit, so this is sized once and never grows.
    ///
    /// An uninitialized slot holds [`Value::empty`].
    slots: RefCell<Box<[Value]>>,
}

impl Globals {
    /// Construct an empty storage which has no slots.
    ///
    /// This is what a [`Vm`] is constructed with. Reading or writing a static
    /// through it always errors, so a unit which declares statics needs real
    /// storage through [`Vm::with_globals`].
    ///
    /// [`Vm`]: crate::runtime::Vm
    /// [`Vm::with_globals`]: crate::runtime::Vm::with_globals
    #[inline]
    pub const fn empty() -> Self {
        Self { inner: None }
    }

    /// Construct storage for all the statics declared by the given unit, with
    /// every slot uninitialized.
    pub fn new(unit: Arc<Unit>) -> alloc::Result<Self> {
        let len = unit.globals_len();

        let mut slots = Vec::try_with_capacity(len)?;

        for _ in 0..len {
            slots.try_push(Value::empty())?;
        }

        Ok(Self {
            inner: Some(Rc::new(GlobalsInner {
                unit,
                slots: RefCell::new(slots.try_into_boxed_slice()?),
            })),
        })
    }

    /// Test if this storage has been configured for a unit.
    ///
    /// Note that this is not the same as [`is_empty`], since a unit which
    /// declares no statics gets configured storage with no slots in it.
    ///
    /// [`is_empty`]: Globals::is_empty
    #[inline]
    pub fn is_configured(&self) -> bool {
        self.inner.is_some()
    }

    /// The number of slots in this storage.
    #[inline]
    pub fn len(&self) -> usize {
        match &self.inner {
            Some(inner) => inner.slots.borrow().len(),
            None => 0,
        }
    }

    /// Test if this storage has no slots.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Test if two handles refer to the same storage.
    #[inline]
    pub fn is_same(&self, other: &Self) -> bool {
        match (&self.inner, &other.inner) {
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        }
    }

    /// Test if this storage was constructed for the given unit.
    #[inline]
    pub fn is_same_unit(&self, unit: &Arc<Unit>) -> bool {
        match &self.inner {
            Some(inner) => Arc::ptr_eq(&inner.unit, unit),
            None => false,
        }
    }

    /// Look up the slot assigned to the given static item.
    pub fn slot(&self, name: impl ToTypeHash) -> Result<usize, GlobalsError> {
        let inner = self.inner()?;
        let hash = name.to_type_hash();

        inner
            .unit
            .global_slot(&hash)
            .ok_or(GlobalsError::new(GlobalsErrorKind::MissingStatic {
                hash,
                item: name.to_item().ok().flatten(),
            }))
    }

    /// Read the value of a static item, if it has been initialized.
    ///
    /// Note that this does not evaluate the initializer of an uninitialized
    /// static, since doing so requires a runtime. A static which has an
    /// initializer but has never been read by a script therefore reads as
    /// `None` here.
    #[inline]
    pub fn get(&self, name: impl ToTypeHash) -> Result<Option<Value>, GlobalsError> {
        let slot = self.slot(name)?;
        Ok(self.get_at(slot))
    }

    /// Write the value of a static item.
    #[inline]
    pub fn set(&self, name: impl ToTypeHash, value: Value) -> Result<(), GlobalsError> {
        let slot = self.slot(name)?;
        self.set_at(slot, value)
    }

    /// Reset a static item back to being uninitialized.
    ///
    /// The next script which reads it will evaluate its initializer again, if
    /// it has one.
    #[inline]
    pub fn clear(&self, name: impl ToTypeHash) -> Result<(), GlobalsError> {
        let slot = self.slot(name)?;
        self.clear_at(slot)
    }

    /// Read the value in the given slot, if it has been initialized.
    #[inline]
    pub fn get_at(&self, slot: usize) -> Option<Value> {
        self.try_get_at(slot).ok().flatten()
    }

    /// Write the value in the given slot.
    #[inline]
    pub fn set_at(&self, slot: usize, value: Value) -> Result<(), GlobalsError> {
        self.replace_at(slot, value)
    }

    /// Reset the given slot back to being uninitialized.
    #[inline]
    pub fn clear_at(&self, slot: usize) -> Result<(), GlobalsError> {
        self.replace_at(slot, Value::empty())
    }

    /// Test if the given slot has been initialized.
    #[inline]
    pub fn is_initialized(&self, slot: usize) -> bool {
        let Some(inner) = &self.inner else {
            return false;
        };

        let slots = inner.slots.borrow();

        match slots.get(slot) {
            Some(value) => !value.is_empty(),
            None => false,
        }
    }

    /// Read the value in the given slot, distinguishing a slot which is out of
    /// bounds from one which is merely uninitialized.
    ///
    /// The virtual machine needs to tell those apart, since the former means
    /// the storage doesn't belong to the unit being run and the latter means
    /// the initializer still has to be evaluated.
    pub(crate) fn try_get_at(&self, slot: usize) -> Result<Option<Value>, GlobalsError> {
        let inner = self.inner()?;
        let slots = inner.slots.borrow();

        let Some(value) = slots.get(slot) else {
            return Err(GlobalsError::new(GlobalsErrorKind::SlotOutOfBounds {
                slot,
                len: slots.len(),
            }));
        };

        if value.is_empty() {
            return Ok(None);
        }

        Ok(Some(value.clone()))
    }

    /// Write a slot, returning an error if it is out of bounds.
    fn replace_at(&self, slot: usize, value: Value) -> Result<(), GlobalsError> {
        let inner = self.inner()?;
        let mut slots = inner.slots.borrow_mut();
        let len = slots.len();

        let Some(existing) = slots.get_mut(slot) else {
            return Err(GlobalsError::new(GlobalsErrorKind::SlotOutOfBounds {
                slot,
                len,
            }));
        };

        *existing = value;
        Ok(())
    }

    /// Deconstruct this handle into its shared allocation.
    #[inline]
    pub(crate) fn into_inner(self) -> Option<Rc<GlobalsInner>> {
        self.inner
    }

    /// Construct a handle from a shared allocation.
    #[inline]
    pub(crate) fn from_inner(inner: Option<Rc<GlobalsInner>>) -> Self {
        Self { inner }
    }

    #[inline]
    fn inner(&self) -> Result<&GlobalsInner, GlobalsError> {
        match &self.inner {
            Some(inner) => Ok(inner),
            None => Err(GlobalsError::new(GlobalsErrorKind::Missing)),
        }
    }
}

impl TryClone for Globals {
    #[inline]
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(self.clone())
    }
}

impl fmt::Debug for Globals {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_struct("Globals");

        match &self.inner {
            Some(inner) => f.field("len", &inner.slots.borrow().len()),
            None => f.field("len", &0usize),
        };

        f.finish_non_exhaustive()
    }
}

/// An error raised when accessing static item storage.
#[derive(Debug)]
pub struct GlobalsError {
    kind: GlobalsErrorKind,
}

impl GlobalsError {
    #[inline]
    const fn new(kind: GlobalsErrorKind) -> Self {
        Self { kind }
    }
}

impl fmt::Display for GlobalsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            GlobalsErrorKind::Missing => {
                write!(f, "No static item storage has been configured")
            }
            GlobalsErrorKind::MissingStatic { hash, item } => match item {
                Some(item) => write!(f, "Missing static item `{item}`"),
                None => write!(f, "Missing static item with hash {hash}"),
            },
            GlobalsErrorKind::SlotOutOfBounds { slot, len } => {
                write!(f, "Static slot {slot} is out of bounds 0-{len}")
            }
        }
    }
}

impl core::error::Error for GlobalsError {}

#[derive(Debug)]
enum GlobalsErrorKind {
    Missing,
    MissingStatic { hash: Hash, item: Option<ItemBuf> },
    SlotOutOfBounds { slot: usize, len: usize },
}
