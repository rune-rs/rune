/// Helper to borrow out a [ResolveContext][crate::parse::ResolveContext].
macro_rules! resolve_context {
    ($q:expr) => {
        $crate::parse::ResolveContext {
            sources: $q.sources,
            storage: $q.storage,
        }
    };
}

/// Call the given macro with repeated type arguments and counts.
macro_rules! repeat_macro {
    ($macro:ident) => {
        $macro!(0);
        $macro!(1, A a 0);
        $macro!(2, A a 0, B b 1);
        $macro!(3, A a 0, B b 1, C c 2);
        $macro!(4, A a 0, B b 1, C c 2, D d 3);
        #[cfg(not(test))]
        $macro!(5, A a 0, B b 1, C c 2, D d 3, E e 4);
        #[cfg(not(test))]
        $macro!(6, A a 0, B b 1, C c 2, D d 3, E e 4, F f 5);
        #[cfg(not(test))]
        $macro!(7, A a 0, B b 1, C c 2, D d 3, E e 4, F f 5, G g 6);
        #[cfg(not(test))]
        $macro!(8, A a 0, B b 1, C c 2, D d 3, E e 4, F f 5, G g 6, H h 7);
        #[cfg(not(test))]
        $macro!(9, A a 0, B b 1, C c 2, D d 3, E e 4, F f 5, G g 6, H h 7, I i 8);
        #[cfg(not(test))]
        $macro!(10, A a 0, B b 1, C c 2, D d 3, E e 4, F f 5, G g 6, H h 7, I i 8, J j 9);
        #[cfg(not(test))]
        $macro!(11, A a 0, B b 1, C c 2, D d 3, E e 4, F f 5, G g 6, H h 7, I i 8, J j 9, K k 10);
        #[cfg(not(test))]
        $macro!(12, A a 0, B b 1, C c 2, D d 3, E e 4, F f 5, G g 6, H h 7, I i 8, J j 9, K k 10, L l 11);
        #[cfg(not(test))]
        $macro!(13, A a 0, B b 1, C c 2, D d 3, E e 4, F f 5, G g 6, H h 7, I i 8, J j 9, K k 10, L l 11, M m 12);
        #[cfg(not(test))]
        $macro!(14, A a 0, B b 1, C c 2, D d 3, E e 4, F f 5, G g 6, H h 7, I i 8, J j 9, K k 10, L l 11, M m 12, N n 13);
        #[cfg(not(test))]
        $macro!(15, A a 0, B b 1, C c 2, D d 3, E e 4, F f 5, G g 6, H h 7, I i 8, J j 9, K k 10, L l 11, M m 12, N n 13, O o 14);
        #[cfg(not(test))]
        $macro!(16, A a 0, B b 1, C c 2, D d 3, E e 4, F f 5, G g 6, H h 7, I i 8, J j 9, K k 10, L l 11, M m 12, N n 13, O o 14, P p 15);
    };
}

macro_rules! cfg_std {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "std")]
            #[cfg_attr(rune_docsrs, doc(cfg(feature = "std")))]
            $item
        )*
    }
}

macro_rules! assert_impl {
    ($ty:ty: $first_trait:ident $(+ $rest_trait:ident)*) => {
        #[cfg(test)]
        const _: () = const {
            const fn assert_traits<T>() where T: $first_trait $(+ $rest_trait)* {}
            assert_traits::<$ty>();
        };
    };
}
