/// impl IntoTokens for a struct.
macro_rules! into_tokens {
    ($ty:ty {$($field:ident),*}) => {
        impl $crate::IntoTokens for $ty {
            fn into_tokens(&self, context: &mut $crate::MacroContext, stream: &mut $crate::TokenStream) {
                $(self.$field.into_tokens(context, stream);)*
            }
        }
    }
}

/// impl IntoTokens for an enum.
macro_rules! into_tokens_enum {
    ($ty:ty {$($variant:ident),*}) => {
        impl $crate::IntoTokens for $ty {
            fn into_tokens(&self, context: &mut $crate::MacroContext, stream: &mut $crate::TokenStream) {
                match self {
                    $(Self::$variant(value) => value.into_tokens(context, stream),)*
                }
            }
        }
    }
}
