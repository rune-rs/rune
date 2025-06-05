use rust_alloc::sync::Arc;

use crate::compile::ContextError;
use crate::function_meta::{
    Associated, AssociatedFunctionData, FunctionData, FunctionMetaKind, ToInstance,
};
use crate::item::IntoComponent;
use crate::module::ItemFnMut;
use crate::runtime::{FunctionHandler, TypeInfo, TypeOf};
use crate::{Hash, ItemBuf};

use super::Module;

/// Raw function builder as returned by [`Module::raw_function`].
///
/// This allows for building a function regularly with
/// [`ModuleRawFunctionBuilder::build`] or statically associate the function
/// with a type through [`ModuleRawFunctionBuilder::build_associated::<T>`].
#[must_use = "Must call one of the build functions, like `build` or `build_associated`"]
pub struct ModuleRawFunctionBuilder<'a, N> {
    pub(super) module: &'a mut Module,
    pub(super) name: N,
    pub(super) handler: Arc<FunctionHandler>,
}

impl<'a, N> ModuleRawFunctionBuilder<'a, N> {
    /// Construct a regular function.
    ///
    /// This register the function as a free function in the module it's
    /// associated with, who's full name is the name of the module extended by
    /// the name of the function.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::{Any, Module};
    ///
    /// let mut m = Module::with_item(["module"])?;
    /// m.raw_function("floob", |_, _, _, _| Ok(())).build()?;
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn build(self) -> Result<ItemFnMut<'a>, ContextError>
    where
        N: IntoComponent,
    {
        let item = ItemBuf::with_item([self.name])?;
        self.module
            .function_from_meta_kind(FunctionMetaKind::Function(FunctionData::from_raw(
                item,
                self.handler,
            )))
    }

    /// Construct a function that is associated with `T`.
    ///
    /// This registers the function as an assocaited function, which can only be
    /// used through the type `T`.
    ///
    /// # Errors
    ///
    /// This function call will cause an error in [`Context::install`] if the
    /// type we're associating it with has not been registered.
    ///
    /// [`Context::install`]: crate::Context::install
    ///
    /// ```
    /// use rune::{Any, Module, Context};
    ///
    /// #[derive(Any)]
    /// struct Thing;
    ///
    /// let mut m = Module::default();
    /// m.function("floob", || ()).build_associated::<Thing>()?;
    ///
    /// let mut c = Context::default();
    /// assert!(c.install(m).is_err());
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::{Any, Module};
    ///
    /// #[derive(Any)]
    /// struct Thing;
    ///
    /// let mut m = Module::default();
    /// m.ty::<Thing>()?;
    /// m.raw_function("floob", |_, _, _, _| Ok(())).build_associated::<Thing>()?;
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn build_associated<T>(self) -> Result<ItemFnMut<'a>, ContextError>
    where
        N: ToInstance,
        T: TypeOf,
    {
        let associated = Associated::from_type::<T>(self.name.to_instance()?)?;

        self.module
            .function_from_meta_kind(FunctionMetaKind::AssociatedFunction(
                AssociatedFunctionData::from_raw(associated, self.handler),
            ))
    }

    /// Construct a function that is associated with a custom dynamically
    /// specified container.
    ///
    /// This registers the function as an assocaited function, which can only be
    /// used through the specified type.
    ///
    /// [`Hash`] and [`TypeInfo`] are usually constructed through the [`TypeOf`]
    /// trait. But that requires access to a static type, for which you should
    /// use [`build_associated`] instead.
    ///
    /// # Errors
    ///
    /// The function call will error if the specified type is not already
    /// registered in the module.
    ///
    /// [`Hash`]: crate::Hash
    /// [`build_associated`]: super::ModuleFunctionBuilder::build_associated
    #[inline]
    pub fn build_associated_with(
        self,
        container: Hash,
        container_type_info: TypeInfo,
    ) -> Result<ItemFnMut<'a>, ContextError>
    where
        N: ToInstance,
    {
        let associated = Associated::new(self.name.to_instance()?, container, container_type_info);
        self.module
            .function_from_meta_kind(FunctionMetaKind::AssociatedFunction(
                AssociatedFunctionData::from_raw(associated, self.handler),
            ))
    }
}
