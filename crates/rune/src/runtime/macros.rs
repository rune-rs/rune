macro_rules! range_iter {
    ($range:ident, $name:ident<$ty:ident> $(, { $($item:tt)* })?) => {
        #[derive(Any)]
        #[rune(item = ::std::ops)]
        pub(crate) struct $name<$ty>
        where
            $ty: 'static + $crate::alloc::clone::TryClone,
            $ty: $crate::compile::Named,
            $ty: $crate::runtime::FromValue + $crate::runtime::ToValue,
            $ty: $crate::runtime::MaybeTypeOf + $crate::runtime::TypeOf,
        {
            iter: core::ops::$range<$ty>,
        }

        impl<$ty> $name<$ty>
        where
            $ty: 'static + $crate::alloc::clone::TryClone,
            $ty: $crate::compile::Named,
            $ty: $crate::runtime::FromValue + $crate::runtime::ToValue,
            $ty: $crate::runtime::MaybeTypeOf + $crate::runtime::TypeOf,
            core::ops::$range<$ty>: Iterator<Item = $ty>,
        {
            #[inline]
            pub(crate) fn new(iter: core::ops::$range<$ty>) -> Self {
                Self { iter }
            }

            #[rune::function(instance, keep, protocol = NEXT)]
            #[inline]
            pub(crate) fn next(&mut self) -> Option<$ty> {
                self.iter.next()
            }

            $($($item)*)*
        }

        impl<$ty> Iterator for $name<$ty>
        where
            $ty: 'static + $crate::alloc::clone::TryClone,
            $ty: $crate::compile::Named,
            $ty: $crate::runtime::FromValue + $crate::runtime::ToValue,
            $ty: $crate::runtime::MaybeTypeOf + $crate::runtime::TypeOf,
            core::ops::$range<$ty>: Iterator<Item = $ty>,
        {
            type Item = $ty;

            #[inline]
            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next()
            }
        }
    };
}

macro_rules! double_ended_range_iter {
    ($range:ident, $name:ident<$ty:ident> $(, { $($item:tt)* })?) => {
        range_iter!($range, $name<$ty> $(, { $($item)* })*);

        impl<T> $name<T>
        where
            T: 'static + $crate::alloc::clone::TryClone,
            T: $crate::compile::Named,
            T: $crate::runtime::FromValue + $crate::runtime::ToValue,
            T: $crate::runtime::MaybeTypeOf + $crate::runtime::TypeOf,
            core::ops::$range<T>: DoubleEndedIterator<Item = T>,
        {
            #[rune::function(instance, keep, protocol = NEXT_BACK)]
            #[inline]
            pub(crate) fn next_back(&mut self) -> Option<T> {
                self.iter.next_back()
            }
        }

        impl<T> DoubleEndedIterator for $name<T>
        where
            T: 'static + $crate::alloc::clone::TryClone,
            T: $crate::compile::Named,
            T: $crate::runtime::FromValue + $crate::runtime::ToValue,
            T: $crate::runtime::MaybeTypeOf + $crate::runtime::TypeOf,
            core::ops::$range<T>: DoubleEndedIterator<Item = T>,
        {
            #[inline]
            fn next_back(&mut self) -> Option<Self::Item> {
                self.iter.next_back()
            }
        }
    };
}
