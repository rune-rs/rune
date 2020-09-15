/// impl IntoTokens for a struct.
macro_rules! into_tokens {
    ($ty:ty {$($field:ident),+}) => {
        impl $crate::IntoTokens for $ty {
            fn into_tokens(&self, context: &mut $crate::MacroContext, stream: &mut $crate::TokenStream) {
                $(self.$field.into_tokens(context, stream);)*
            }
        }
    };

    ($ty:ty {}) => {
        impl $crate::IntoTokens for $ty {
            fn into_tokens(&self, _: &mut $crate::MacroContext, _: &mut $crate::TokenStream) {
            }
        }
    };
}

/// Implement an enum with variants containing AST.
macro_rules! impl_enum_ast {
    (
        $(#[$meta:meta])*
        pub enum $name:ident {
            $(
                $(#[$v_meta:meta])*
                $v_name:ident ($v_ty:ty),
            )*
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone)]
        pub enum $name {
            $($(#[$v_meta])* $v_name ($v_ty),)*
        }

        impl $crate::IntoTokens for $name {
            fn into_tokens(&self, context: &mut $crate::MacroContext, stream: &mut $crate::TokenStream) {
                match self {
                    $(Self::$v_name(value) => value.into_tokens(context, stream),)*
                }
            }
        }

        impl $crate::Spanned for $name {
            fn span(&self) -> runestick::Span {
                match self {
                    $(Self::$v_name(v) => v.span(),)*
                }
            }
        }
    }
}
