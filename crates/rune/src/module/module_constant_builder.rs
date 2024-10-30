use crate::compile::ContextError;
use crate::function_meta::{Associated, ToInstance};
use crate::item::IntoComponent;
use crate::module::ItemMut;
use crate::runtime::{ToConstValue, TypeHash, TypeOf};

use super::Module;

/// Raw function builder as returned by [`Module::raw_function`].
///
/// This allows for building a function regularly with
/// [`ModuleConstantBuilder::build`] or statically associate the function with a
/// type through [`ModuleConstantBuilder::build_associated::<T>`].
#[must_use = "Must call one of the build functions, like `build` or `build_associated`"]
pub struct ModuleConstantBuilder<'a, N, V> {
    pub(super) module: &'a mut Module,
    pub(super) name: N,
    pub(super) value: V,
}

impl<'a, N, V> ModuleConstantBuilder<'a, N, V>
where
    V: TypeHash + TypeOf + ToConstValue,
{
    /// Add the free constant directly to the module.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::{Any, Module};
    /// use rune::runtime::VmResult;
    ///
    /// let mut m = Module::with_item(["module"])?;
    /// m.constant("NAME", "Hello World").build()?;
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn build(self) -> Result<ItemMut<'a>, ContextError>
    where
        N: IntoComponent,
    {
        self.module.insert_constant(self.name, self.value)
    }

    /// Build a constant that is associated with the static type `T`.
    ///
    /// # Errors
    ///
    /// This function call will cause an error in [`Context::install`] if the
    /// type we're associating it with has not been registered.
    ///
    /// [`Context::install`]: crate::Context::install
    ///
    /// ```
    /// use rune::{Any, Context, Module};
    ///
    /// #[derive(Any)]
    /// struct Thing;
    ///
    /// let mut m = Module::default();
    /// m.constant("CONSTANT", "Hello World").build_associated::<Thing>()?;
    ///
    /// let mut c = Context::default();
    /// assert!(c.install(m).is_err());
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::{docstring, Any, Module};
    ///
    /// let mut module = Module::default();
    ///
    /// #[derive(Any)]
    /// struct Thing;
    ///
    /// module.constant("TEN", 10)
    ///     .build_associated::<Thing>()?
    ///     .docs(docstring! {
    ///         /// Ten which is an associated constant.
    ///     });
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn build_associated<T>(self) -> Result<ItemMut<'a>, ContextError>
    where
        T: TypeOf,
        N: ToInstance,
    {
        let name = self.name.to_instance()?;
        let associated = Associated::from_type::<T>(name)?;
        self.module
            .insert_associated_constant(associated, self.value)
    }
}
