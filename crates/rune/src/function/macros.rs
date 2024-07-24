// Note: Automatically generated using function_traits_permute.py
macro_rules! permute {
    ($call:path) => {
        $call!(0);
        $call!(1, {A, a, A, 0, {}, {FromValue}, from_value});
        $call!(1, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        $call!(1, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        $call!(2, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value});
        $call!(2, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        $call!(2, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        $call!(2, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value});
        $call!(2, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        $call!(2, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        $call!(2, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value});
        $call!(2, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        $call!(2, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        $call!(3, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value});
        $call!(3, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        $call!(3, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        $call!(3, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value});
        $call!(3, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        $call!(3, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        $call!(3, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value});
        $call!(3, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        $call!(3, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        $call!(3, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value});
        $call!(3, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        $call!(3, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        $call!(3, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value});
        $call!(3, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        $call!(3, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        $call!(3, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value});
        $call!(3, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        $call!(3, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        $call!(3, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value});
        $call!(3, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        $call!(3, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        $call!(3, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value});
        $call!(3, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        $call!(3, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        $call!(3, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value});
        $call!(3, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        $call!(3, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(4, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(4, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(4, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(4, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(4, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(4, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(4, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(4, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(4, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(4, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(4, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(4, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(4, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(4, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(4, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(4, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(4, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(4, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(4, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(4, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(4, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(4, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(4, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(4, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(4, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(4, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(4, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(4, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(4, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(4, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(4, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(4, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(4, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(4, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(4, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(4, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(4, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(4, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(4, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(4, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(4, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(4, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(4, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(4, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(4, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(4, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(4, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(4, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(4, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(4, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(4, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(4, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(4, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(4, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(4, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(4, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(4, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(4, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(4, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(4, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(4, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(4, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(4, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(4, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(4, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(4, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(4, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(4, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(4, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(4, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(4, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(4, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(4, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(4, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(4, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(4, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(4, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(4, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(4, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(4, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(4, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, A, 0, {}, {FromValue}, from_value}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Ref<A>, 0, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, B, 1, {}, {FromValue}, from_value}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Ref<B>, 1, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, C, 2, {}, {FromValue}, from_value}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Ref<C>, 2, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, D, 3, {}, {FromValue}, from_value}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Ref<D>, 3, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, E, 4, {}, {FromValue}, from_value});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Ref<E>, 4, {&}, {?Sized + UnsafeToRef}, unsafe_to_ref});
        #[cfg(not(test))]
        $call!(5, {A, a, Mut<A>, 0, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {B, b, Mut<B>, 1, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {C, c, Mut<C>, 2, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {D, d, Mut<D>, 3, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut}, {E, e, Mut<E>, 4, {&mut}, {?Sized + UnsafeToMut}, unsafe_to_mut});
    }
}
