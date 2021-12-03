/// Helper macro to construct an error type.
macro_rules! error {
    (
        $(#[$meta:meta])*
        $vis:vis struct $error_ty:ident {
            kind: $kind:ident,
        }

        $(impl From<$from_error:ident>;)*
    ) => {
        $(#[$meta])*
        $vis struct $error_ty {
            span: $crate::ast::Span,
            kind: Box<$kind>,
        }

        impl $error_ty {
            /// Construct a new scope error.
            pub fn new<S, K>(spanned: S, kind: K) -> Self
            where
                S: $crate::ast::Spanned,
                $kind: From<K>,
            {
                Self {
                    span: $crate::ast::Spanned::span(&spanned),
                    kind: Box::new($kind::from(kind)),
                }
            }

            /// Construct an custom error.
            ///
            /// This should be used for programming invariants of the encoder which are
            /// broken for some reason.
            pub fn msg<S>(spanned: S, message: &'static str) -> Self
            where
                S: $crate::ast::Spanned,
            {
                Self::new(spanned, $kind::Custom { message })
            }

            /// Get the kind of the error.
            pub fn kind(&self) -> &$kind {
                &*self.kind
            }

            /// Convert into the kind of the error.
            pub fn into_kind(self) -> $kind {
                *self.kind
            }
        }

        impl $crate::ast::Spanned for $error_ty {
            fn span(&self) -> $crate::ast::Span {
                self.span
            }
        }

        impl std::error::Error for $error_ty {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                self.kind.source()
            }
        }

        impl std::fmt::Display for $error_ty {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(&self.kind, f)
            }
        }

        impl From<$crate::shared::Custom> for $error_ty {
            fn from(error: $crate::shared::Custom) -> Self {
                use $crate::ast::Spanned;

                Self::new(
                    error.span(),
                    $kind::Custom {
                        message: error.message(),
                    },
                )
            }
        }

        $(
            impl From<$from_error> for $error_ty {
                fn from(error: $from_error) -> Self {
                    $error_ty {
                        span: error.span(),
                        kind: Box::new($kind::$from_error {
                            error: From::from(error.into_kind()),
                        }),
                    }
                }
            }
        )*
    }
}

/// Helper to borrow out a [ResolveContext][crate::parse::ResolveContext].
macro_rules! resolve_context {
    ($q:expr) => {
        $crate::parse::ResolveContext {
            sources: $q.sources,
            storage: $q.storage,
        }
    };
}

/// Build an implementation of `TypeOf` basic of a static type.
macro_rules! impl_static_type {
    (impl <$($p:ident),*> $ty:ty => $static_type:expr) => {
        impl<$($p,)*> $crate::runtime::TypeOf for $ty {
            fn type_hash() -> $crate::Hash {
                $static_type.hash
            }

            fn type_info() -> $crate::runtime::TypeInfo {
                $crate::runtime::TypeInfo::StaticType($static_type)
            }
        }
    };

    ($ty:ty => $static_type:expr) => {
        impl $crate::runtime::TypeOf for $ty {
            fn type_hash() -> $crate::Hash {
                $static_type.hash
            }

            fn type_info() -> $crate::runtime::TypeInfo {
                $crate::runtime::TypeInfo::StaticType($static_type)
            }
        }
    };
}

/// Call the given macro with repeated type arguments and counts.
macro_rules! repeat_macro {
    ($macro:tt) => {
        $macro! {
            {A, a, 16},
            {B, b, 15},
            {C, c, 14},
            {D, d, 13},
            {E, e, 12},
            {F, f, 11},
            {G, g, 10},
            {H, h, 9},
            {I, i, 8},
            {J, j, 7},
            {K, k, 6},
            {L, l, 5},
            {M, m, 4},
            {N, n, 3},
            {O, o, 2},
            {P, p, 1},
        }
    };
}

macro_rules! cfg_emit {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "emit")]
            #[cfg_attr(docsrs, doc(cfg(feature = "emit")))]
            $item
        )*
    }
}

macro_rules! cfg_workspace {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "workspace")]
            #[cfg_attr(docsrs, doc(cfg(feature = "workspace")))]
            $item
        )*
    }
}
