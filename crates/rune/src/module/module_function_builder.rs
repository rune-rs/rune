use crate::compile::ContextError;
use crate::function::{Function, FunctionKind};
use crate::function_meta::{FunctionArgs, FunctionBuilder, ToInstance};
use crate::item::IntoComponent;
use crate::module::ItemFnMut;
use crate::runtime::{MaybeTypeOf, TypeInfo, TypeOf};
use crate::Hash;

use super::Module;

/// Function builder as returned by [`Module::function`].
///
/// This allows for building a function regularly with
/// [`ModuleFunctionBuilder::build`] or statically associate the function with a
/// type through [`ModuleFunctionBuilder::build_associated::<T>`].
#[must_use = "Must call one of the build functions, like `build` or `build_associated`"]
pub struct ModuleFunctionBuilder<'a, F, A, N, K> {
    pub(super) module: &'a mut Module,
    pub(super) inner: FunctionBuilder<N, F, A, K>,
}

impl<'a, F, A, N, K> ModuleFunctionBuilder<'a, F, A, N, K>
where
    F: Function<A, K>,
    F::Return: MaybeTypeOf,
    A: FunctionArgs,
    K: FunctionKind,
{
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
    /// m.function("floob", || ()).build()?;
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn build(self) -> Result<ItemFnMut<'a>, ContextError>
    where
        N: IntoComponent,
    {
        let meta = self.inner.build()?;
        self.module.function_from_meta_kind(meta)
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
    /// use rune::{Any, Context, Module};
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
    /// m.function("floob", || ()).build_associated::<Thing>()?;
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn build_associated<T>(self) -> Result<ItemFnMut<'a>, ContextError>
    where
        N: ToInstance,
        T: TypeOf,
    {
        let meta = self.inner.build_associated::<T>()?;
        self.module.function_from_meta_kind(meta)
    }

    /// Construct a function that is associated with a custom dynamically
    /// specified container.
    ///
    /// This registers the function as an assocaited function, which can only be
    /// used through the specified type.
    ///
    /// [`Hash`] and [`TypeInfo`] are usually constructed through the
    /// [`TypeOf`] trait. But that requires access to a static type, for which
    /// you should use [`build_associated`] instead.
    ///
    /// # Errors
    ///
    /// The function call will error if the specified type is not already
    /// registered in the module.
    ///
    /// [`build_associated`]: ModuleFunctionBuilder::build_associated
    /// [`Hash`]: crate::Hash
    #[inline]
    pub fn build_associated_with(
        self,
        container: Hash,
        container_type_info: TypeInfo,
    ) -> Result<ItemFnMut<'a>, ContextError>
    where
        N: ToInstance,
    {
        let meta = self
            .inner
            .build_associated_with(container, container_type_info)?;
        self.module.function_from_meta_kind(meta)
    }
}
