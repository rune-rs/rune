macro_rules! inline_into {
    (
        $(#[$($meta:meta)*])*
        $kind:ident($ty:ty),
        $as:ident,
        $as_mut:ident,
    ) => {
        $(#[$($meta)*])*
        ///
        /// This gets a copy of the value.
        #[inline]
        pub fn $as(&self) -> Result<$ty, RuntimeError> {
            match &self.kind {
                ConstValueKind::Inline(Inline::$kind(value)) => {
                    Ok(*value)
                }
                value => {
                    Err(RuntimeError::expected::<$ty>(value.type_info()))
                }
            }
        }

        $(#[$($meta)*])*
        ///
        /// This gets the value by mutable reference.
        #[inline]
        pub fn $as_mut(&mut self) -> Result<&mut $ty, RuntimeError> {
            match &mut self.kind {
                ConstValueKind::Inline(Inline::$kind(value)) => {
                    Ok(value)
                }
                value => {
                    Err(RuntimeError::expected::<$ty>(value.type_info()))
                }
            }
        }
    }
}
