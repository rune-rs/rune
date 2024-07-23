macro_rules! range_iter {
    ($range:ident, $name:ident) => {
        #[derive(Any)]
        #[rune(item = ::std::ops)]
        pub(crate) struct $name<T>
        where
            T: 'static + $crate::alloc::clone::TryClone,
            T: $crate::compile::Named,
            T: $crate::runtime::FromValue + $crate::runtime::ToValue,
            T: $crate::runtime::MaybeTypeOf + $crate::runtime::TypeOf,
        {
            iter: ::core::ops::$range<T>,
        }

        impl<T> $name<T>
        where
            T: 'static + $crate::alloc::clone::TryClone,
            T: $crate::compile::Named,
            T: $crate::runtime::FromValue + $crate::runtime::ToValue,
            T: $crate::runtime::MaybeTypeOf + $crate::runtime::TypeOf,
            ::core::ops::$range<T>: Iterator<Item = T>,
        {
            #[inline]
            pub(crate) fn new(iter: ::core::ops::$range<T>) -> Self {
                Self { iter }
            }

            #[rune::function(instance, keep, protocol = NEXT)]
            #[inline]
            pub(crate) fn next(&mut self) -> Option<T> {
                self.iter.next()
            }

            #[rune::function(instance, keep, protocol = INTO_ITER)]
            #[inline]
            pub(crate) fn into_iter(self) -> Self {
                self
            }
        }

        impl<T> Iterator for $name<T>
        where
            T: 'static + $crate::alloc::clone::TryClone,
            T: $crate::compile::Named,
            T: $crate::runtime::FromValue + $crate::runtime::ToValue,
            T: $crate::runtime::MaybeTypeOf + $crate::runtime::TypeOf,
            ::core::ops::$range<T>: Iterator<Item = T>,
        {
            type Item = T;

            #[inline]
            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next()
            }
        }
    };
}

macro_rules! double_ended_range_iter {
    ($range:ident, $name:ident) => {
        range_iter!($range, $name);

        impl<T> $name<T>
        where
            T: 'static + $crate::alloc::clone::TryClone,
            T: $crate::compile::Named,
            T: $crate::runtime::FromValue + $crate::runtime::ToValue,
            T: $crate::runtime::MaybeTypeOf + $crate::runtime::TypeOf,
            ::core::ops::$range<T>: DoubleEndedIterator<Item = T>,
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
            ::core::ops::$range<T>: DoubleEndedIterator<Item = T>,
        {
            #[inline]
            fn next_back(&mut self) -> Option<Self::Item> {
                self.iter.next_back()
            }
        }
    };
}
