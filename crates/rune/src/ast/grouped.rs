use core::slice;

use crate::no_std::vec;

use crate::ast::prelude::*;

macro_rules! grouped {
    ($(#[$meta:meta])* $name:ident { $field:ident, $open:ty, $close:ty }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Spanned, ToTokens)]
        #[non_exhaustive]
        pub struct $name<T, S> {
            /// The open parenthesis.
            pub open: $open,
            /// Values in the type.
            pub $field: Vec<(T, Option<S>)>,
            /// The close parenthesis.
            pub close: $close,
        }

        impl<T, S> $name<T, S> {
            /// Test if group is empty.
            pub fn is_empty(&self) -> bool {
                self.$field.is_empty()
            }

            /// Get the length of elements in the group.
            pub fn len(&self) -> usize {
                self.$field.len()
            }

            /// Get the first element in the group.
            pub fn first(&self) -> Option<&(T, Option<S>)> {
                self.$field.first()
            }

            /// Get the last element in the group.
            pub fn last(&self) -> Option<&(T, Option<S>)> {
                self.$field.last()
            }

            /// Iterate over elements in the group.
            pub fn iter(&self) -> slice::Iter<'_, (T, Option<S>)> {
                self.$field.iter()
            }

            /// Iterate mutably over elements in the group.
            pub fn iter_mut(&mut self) -> slice::IterMut<'_, (T, Option<S>)> {
                self.$field.iter_mut()
            }

            /// Get the group values as a slice.
            pub fn as_slice(&self) -> &[(T, Option<S>)] {
                &*self.$field
            }

            /// Get the group values as a mutable slice.
            pub fn as_mut(&mut self) -> &mut [(T, Option<S>)] {
                &mut *self.$field
            }
        }

        impl<'a, T, S> IntoIterator for &'a $name<T, S> {
            type Item = &'a (T, Option<S>);
            type IntoIter = slice::Iter<'a, (T, Option<S>)>;

            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }

        impl<'a, T, S> IntoIterator for &'a mut $name<T, S> {
            type Item = &'a mut (T, Option<S>);
            type IntoIter = slice::IterMut<'a, (T, Option<S>)>;

            fn into_iter(self) -> Self::IntoIter {
                self.iter_mut()
            }
        }

        impl<T, S> IntoIterator for $name<T, S> {
            type Item = (T, Option<S>);
            type IntoIter = vec::IntoIter<(T, Option<S>)>;

            fn into_iter(self) -> Self::IntoIter {
                self.$field.into_iter()
            }
        }

        impl<T, S> $name<T, S>
        where
            T: Parse,
            S: Peek + Parse,
        {
            /// Parse with the first element already specified.
            pub fn parse_from_first(
                parser: &mut Parser<'_>,
                open: $open,
                mut current: T,
            ) -> Result<Self> {
                let mut $field = Vec::new();

                loop {
                    let comma = parser.parse::<Option<S>>()?;
                    let is_end = comma.is_none();
                    $field.push((current, comma));

                    if is_end || parser.peek::<$close>()? {
                        break;
                    }

                    current = parser.parse()?;
                }

                let close = parser.parse()?;

                Ok(Self {
                    open,
                    $field,
                    close,
                })
            }
        }

        impl<T, S> Parse for $name<T, S>
        where
            T: Parse,
            S: Peek + Parse,
        {
            fn parse(parser: &mut Parser<'_>) -> Result<Self> {
                let open = parser.parse()?;

                let mut $field = Vec::new();

                while !parser.peek::<$close>()? {
                    let expr = parser.parse()?;
                    let sep = parser.parse::<Option<S>>()?;
                    let is_end = sep.is_none();
                    $field.push((expr, sep));

                    if is_end {
                        break;
                    }
                }

                let close = parser.parse()?;

                Ok(Self {
                    open,
                    $field,
                    close,
                })
            }
        }

        impl<T, S> Peek for $name<T, S> {
            fn peek(p: &mut Peeker<'_>) -> bool {
                <$open>::peek(p)
            }
        }
    }
}

grouped! {
    /// Parse something parenthesis, that is separated by `((T, S?)*)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::{T, testing, ast};
    ///
    /// testing::roundtrip::<ast::Parenthesized<ast::Expr, T![,]>>("(1, \"two\")");
    /// testing::roundtrip::<ast::Parenthesized<ast::Expr, T![,]>>("(1, 2,)");
    /// testing::roundtrip::<ast::Parenthesized<ast::Expr, T![,]>>("(1, 2, foo())");
    /// ```
    Parenthesized { parenthesized, ast::OpenParen, ast::CloseParen }
}

grouped! {
    /// Parse something bracketed, that is separated by `[(T, S?)*]`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::{T, testing, ast};
    ///
    /// testing::roundtrip::<ast::Bracketed<ast::Expr, T![,]>>("[1, \"two\"]");
    /// testing::roundtrip::<ast::Bracketed<ast::Expr, T![,]>>("[1, 2,]");
    /// testing::roundtrip::<ast::Bracketed<ast::Expr, T![,]>>("[1, 2, foo()]");
    /// ```
    Bracketed { bracketed, ast::OpenBracket, ast::CloseBracket }
}

grouped! {
    /// Parse something braced, that is separated by `{(T, S?)*}`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::{T, testing, ast};
    ///
    /// testing::roundtrip::<ast::Braced<ast::Expr, T![,]>>("{1, \"two\"}");
    /// testing::roundtrip::<ast::Braced<ast::Expr, T![,]>>("{1, 2,}");
    /// testing::roundtrip::<ast::Braced<ast::Expr, T![,]>>("{1, 2, foo()}");
    /// ```
    Braced { braced, ast::OpenBrace, ast::CloseBrace }
}

grouped! {
    /// Parse something bracketed, that is separated by `<(T, S?)*>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::{T, testing, ast};
    ///
    /// testing::roundtrip::<ast::AngleBracketed<ast::Path, T![,]>>("<Foo, Bar>");
    /// testing::roundtrip::<ast::AngleBracketed<ast::PathSegmentExpr, T![,]>>("<1, \"two\">");
    /// testing::roundtrip::<ast::AngleBracketed<ast::PathSegmentExpr, T![,]>>("<1, 2,>");
    /// testing::roundtrip::<ast::AngleBracketed<ast::PathSegmentExpr, T![,]>>("<1, 2, foo()>");
    /// ```
    AngleBracketed { angle_bracketed, ast::generated::Lt, ast::generated::Gt }
}
