use bevy::prelude::Mut;

impl<T: crate::compile::Named> crate::compile::Named for Mut<'_, T> {
    const BASE_NAME: rune_core::RawStr = T::BASE_NAME;
}

impl<T: crate::Any> crate::Any for Mut<'static, T> {
    fn type_hash() -> rune_core::Hash {
        T::type_hash()
    }
}

/// This is for internal use, need to create a second trait so that way
/// this trait can be implemented for *all* T, without causing the compiler
/// to cry about other types that implement the other UnsafeToValue
pub trait UnsafeToValue2: Sized {
    /// Convert into a value.
    ///
    /// # Safety
    ///
    /// The value returned must not be used after the guard associated with it
    /// has been dropped.
    unsafe fn unsafe_to_value(
        self,
    ) -> crate::runtime::VmResult<(crate::Value, crate::runtime::SharedPointerGuard)>;
}

impl<T: crate::__private::InstallWith> crate::__private::InstallWith for Mut<'static, T> {
    fn install_with(
        module: &mut crate::__private::Module,
    ) -> core::result::Result<(), crate::compile::ContextError> {
        T::install_with(module)?;
        Ok(())
    }
}

impl<T: crate::Any> UnsafeToValue2 for Mut<'_, T> {
    unsafe fn unsafe_to_value(
        self,
    ) -> crate::runtime::VmResult<(crate::runtime::Value, crate::runtime::SharedPointerGuard)> {
        let this: Mut<T> = unsafe { std::mem::transmute(self) };
        let (shared, guard) =
            match crate::runtime::try_result(crate::runtime::Shared::from_bevy_mut(this)) {
                crate::runtime::VmResult::Ok(value) => value,
                crate::runtime::VmResult::Err(err) => {
                    return crate::runtime::VmResult::Err(crate::runtime::VmError::from(err));
                }
            };
        crate::runtime::VmResult::Ok((crate::runtime::Value::from(shared), guard))
    }
}
