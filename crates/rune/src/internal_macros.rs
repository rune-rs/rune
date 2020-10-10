/// Helper macro to construct an error type.
macro_rules! error {
    (
        $(#[$meta:meta])*
        $vis:vis struct $error:ident {
            kind: $kind:ident,
        }

        $(impl From<$from_error:ident>;)*
    ) => {
        $(#[$meta])*
        $vis struct $error {
            span: runestick::Span,
            kind: Box<$kind>,
        }

        impl $error {
            /// Construct a new scope error.
            pub fn new<S, K>(spanned: S, kind: K) -> Self
            where
                S: crate::Spanned,
                $kind: From<K>,
            {
                Self {
                    span: crate::Spanned::span(&spanned),
                    kind: Box::new($kind::from(kind)),
                }
            }

            /// Construct an custom error.
            ///
            /// This should be used for programming invariants of the encoder which are
            /// broken for some reason.
            pub fn msg<S>(spanned: S, message: &'static str) -> Self
            where
                S: Spanned,
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

        impl crate::Spanned for $error {
            fn span(&self) -> runestick::Span {
                self.span
            }
        }

        impl std::error::Error for $error {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                self.kind.source()
            }
        }

        impl std::fmt::Display for $error {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(&self.kind, f)
            }
        }

        impl From<$crate::shared::Custom> for $error {
            fn from(error: $crate::shared::Custom) -> Self {
                Self::new(
                    error.span(),
                    $kind::Custom {
                        message: error.message(),
                    },
                )
            }
        }

        $(
            impl From<$from_error> for $error {
                fn from(error: $from_error) -> Self {
                    $error {
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
