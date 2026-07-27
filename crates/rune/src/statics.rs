use core::fmt;

use crate::alloc::{self, Vec};
use crate::item::IntoComponent;
use crate::{Item, ItemBuf};

/// A collection of statics to declare when building a unit.
///
/// A `static` item usually comes from the source being compiled, but a host
/// which wants to hand a script a piece of state does not necessarily want that
/// state to be declared by the script. Statics declared here are added to the
/// unit as if the source had declared them, so a script can read and write them
/// by name without declaring them itself.
///
/// Such a static has no initializer, since there is no source to evaluate. This
/// means it starts out uninitialized and reading it before anything has
/// assigned it is a runtime error, exactly like a `static N;` in a script. The
/// caller is expected to populate it through [`Globals::set`] before running
/// anything which reads it.
///
/// Declaring a name which the source also declares is an error, since it is
/// ambiguous which of the two declarations a use refers to.
///
/// [`Globals::set`]: crate::runtime::Globals::set
///
/// # Examples
///
/// ```
/// use rune::{Context, Source, Sources, Statics, Vm};
/// use rune::runtime::Globals;
/// use rune::sync::Arc;
///
/// let context = Context::with_default_modules()?;
/// let runtime = Arc::try_new(context.runtime()?)?;
///
/// let mut sources = Sources::new();
/// sources.insert(Source::memory(r#"
/// pub fn main() {
///     LIMIT += 1;
///     LIMIT
/// }
/// "#)?)?;
///
/// let mut statics = Statics::new();
/// statics.insert(["LIMIT"])?;
///
/// let unit = rune::prepare(&mut sources)
///     .with_context(&context)
///     .with_statics(&statics)
///     .build()?;
///
/// let unit = Arc::try_new(unit)?;
/// let globals = Globals::new(unit.clone())?;
/// globals.set(["LIMIT"], rune::to_value(41i64)?)?;
///
/// let mut vm = Vm::new(runtime, unit).with_globals(globals);
/// let output = vm.call(["main"], ())?;
/// assert_eq!(rune::from_value::<i64>(output)?, 42);
/// # Ok::<_, rune::support::Error>(())
/// ```
#[derive(Default)]
pub struct Statics {
    statics: Vec<Static>,
}

impl Statics {
    /// Construct an empty collection of statics.
    #[inline]
    pub const fn new() -> Self {
        Self {
            statics: Vec::new(),
        }
    }

    /// Declare a static with the given name.
    ///
    /// The name is the item the static is declared as, so `["LIMIT"]` declares
    /// it at the root of the unit and `["config", "LIMIT"]` declares it inside
    /// of the `config` module. Every component has to be a valid identifier.
    ///
    /// Declaring the same name twice does nothing the second time.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::Statics;
    ///
    /// let mut statics = Statics::new();
    /// statics.insert(["LIMIT"])?;
    /// statics.insert(["config", "LIMIT"])?;
    /// assert_eq!(statics.len(), 2);
    ///
    /// statics.insert(["LIMIT"])?;
    /// assert_eq!(statics.len(), 2);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn insert(&mut self, name: impl IntoIterator<Item: IntoComponent>) -> alloc::Result<()> {
        let item = ItemBuf::with_item(name)?;

        if self.statics.iter().any(|s| s.item == item) {
            return Ok(());
        }

        self.statics.try_push(Static { item })?;
        Ok(())
    }

    /// The number of statics being declared.
    #[inline]
    pub fn len(&self) -> usize {
        self.statics.len()
    }

    /// Test if the collection is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.statics.is_empty()
    }

    /// Iterate over the statics being declared.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Static> {
        self.statics.iter()
    }
}

impl fmt::Debug for Statics {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.statics.iter()).finish()
    }
}

/// A single static being declared.
///
/// This is a type of its own so that a declaration can carry more than its name
/// in the future, such as the type it is expected to hold.
pub(crate) struct Static {
    item: ItemBuf,
}

impl Static {
    /// The item the static is declared as.
    #[inline]
    pub(crate) fn item(&self) -> &Item {
        &self.item
    }
}

impl fmt::Debug for Static {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.item.fmt(f)
    }
}
