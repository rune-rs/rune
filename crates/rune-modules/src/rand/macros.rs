#[cfg(any(feature = "small_rng", feature = "std_rng"))]
macro_rules! seedable_rng {
    ($m:ident, $ty:ident) => {{
        use rune::runtime::{TypeHash, Value, VmResult};
        use rune::{vm_panic, vm_try};

        $m.function_meta(from_rng)?;
        $m.function_meta(try_from_rng)?;
        #[cfg(feature = "os_rng")]
        $m.function_meta(from_os_rng)?;
        #[cfg(feature = "os_rng")]
        $m.function_meta(try_from_os_rng)?;
        $m.function_meta(from_seed)?;
        $m.function_meta(seed_from_u64)?;

        /// Create a new PRNG seeded from an infallible `Rng`.
        ///
        /// This may be useful when needing to rapidly seed many PRNGs from a master
        /// PRNG, and to allow forking of PRNGs. It may be considered deterministic.
        ///
        /// The master PRNG should be at least as high quality as the child PRNGs.
        /// When seeding non-cryptographic child PRNGs, we recommend using a
        /// different algorithm for the master PRNG (ideally a CSPRNG) to avoid
        /// correlations between the child PRNGs. If this is not possible (e.g.
        /// forking using small non-crypto PRNGs) ensure that your PRNG has a good
        /// mixing function on the output or consider use of a hash function with
        /// `from_seed`.
        ///
        /// Note that seeding `XorShiftRng` from another `XorShiftRng` provides an
        /// extreme example of what can go wrong: the new PRNG will be a clone
        /// of the parent.
        ///
        /// PRNG implementations are allowed to assume that a good RNG is provided
        /// for seeding, and that it is cryptographically secure when appropriate.
        /// As of `rand` 0.7 / `rand_core` 0.5, implementations overriding this
        /// method should ensure the implementation satisfies reproducibility
        /// (in prior versions this was not required).
        ///
        /// [`rand`]: self
        #[rune::function(free, path = $ty::from_rng)]
        fn from_rng(rng: Value) -> VmResult<$ty> {
            match rng.type_hash() {
                #[cfg(feature = "small_rng")]
                crate::rand::SmallRng::HASH => {
                    let mut rng = vm_try!(rng.borrow_mut::<crate::rand::SmallRng>());

                    let inner = match rand::SeedableRng::try_from_rng(&mut rng.inner) {
                        Ok(inner) => inner,
                        Err(error) => return VmResult::panic(error),
                    };

                    VmResult::Ok($ty { inner })
                }
                #[cfg(feature = "std_rng")]
                crate::rand::StdRng::HASH => {
                    let mut rng = vm_try!(rng.borrow_mut::<crate::rand::StdRng>());

                    let inner = match rand::SeedableRng::try_from_rng(&mut rng.inner) {
                        Ok(inner) => inner,
                        Err(error) => return VmResult::panic(error),
                    };

                    VmResult::Ok($ty { inner })
                }
                #[cfg(feature = "thread_rng")]
                crate::rand::ThreadRng::HASH => {
                    let mut rng = vm_try!(rng.borrow_mut::<crate::rand::ThreadRng>());

                    let inner = match rand::SeedableRng::try_from_rng(&mut rng.inner) {
                        Ok(inner) => inner,
                        Err(error) => return VmResult::panic(error),
                    };

                    VmResult::Ok($ty { inner })
                }
                #[cfg(feature = "os_rng")]
                crate::rand::OsRng::HASH => {
                    let mut rng = vm_try!(rng.borrow_mut::<crate::rand::OsRng>());

                    let inner = match rand::SeedableRng::try_from_rng(&mut rng.inner) {
                        Ok(inner) => inner,
                        Err(error) => return VmResult::panic(error),
                    };

                    VmResult::Ok($ty { inner })
                }
                _ => VmResult::panic("expected an rng source"),
            }
        }

        /// Create a new PRNG seeded from a potentially fallible `Rng`.
        ///
        /// See [`from_rng`][$ty::from_rng] docs for more information.
        #[rune::function(free, vm_result, path = $ty::try_from_rng)]
        fn try_from_rng(rng: Value) -> Result<$ty, TryFromRngError> {
            match rng.type_hash() {
                #[cfg(feature = "small_rng")]
                crate::rand::SmallRng::HASH => {
                    let mut rng = rng.borrow_mut::<crate::rand::SmallRng>().vm?;
                    let inner = rand::SeedableRng::try_from_rng(&mut rng.inner)?;
                    Ok($ty { inner })
                }
                #[cfg(feature = "std_rng")]
                crate::rand::StdRng::HASH => {
                    let mut rng = rng.borrow_mut::<crate::rand::StdRng>().vm?;
                    let inner = rand::SeedableRng::try_from_rng(&mut rng.inner)?;
                    Ok($ty { inner })
                }
                #[cfg(feature = "thread_rng")]
                crate::rand::ThreadRng::HASH => {
                    let mut rng = rng.borrow_mut::<crate::rand::ThreadRng>().vm?;
                    let inner = rand::SeedableRng::try_from_rng(&mut rng.inner)?;
                    Ok($ty { inner })
                }
                #[cfg(feature = "os_rng")]
                crate::rand::OsRng::HASH => {
                    let mut rng = rng.borrow_mut::<crate::rand::OsRng>().vm?;
                    let inner = rand::SeedableRng::try_from_rng(&mut rng.inner)?;
                    Ok($ty { inner })
                }
                _ => {
                    vm_panic!("expected an rng source")
                }
            }
        }

        /// Creates a new instance of the RNG seeded via [`getrandom`].
        ///
        /// This method is the recommended way to construct non-deterministic PRNGs
        /// since it is convenient and secure.
        ///
        /// Note that this method may panic on (extremely unlikely) [`getrandom`]
        /// errors. If it's not desirable, use the [`try_from_os_rng`] method
        /// instead.
        ///
        /// # Panics
        ///
        /// If [`getrandom`] is unable to provide secure entropy this method will
        /// panic.
        ///
        /// [`getrandom`]: https://docs.rs/getrandom
        /// [`try_from_os_rng`]: StdRng::try_from_os_rng
        #[rune::function(free, path = $ty::from_os_rng)]
        #[cfg(feature = "os_rng")]
        fn from_os_rng() -> VmResult<$ty> {
            match rand::SeedableRng::try_from_os_rng() {
                Ok(inner) => VmResult::Ok($ty { inner }),
                Err(e) => VmResult::panic(e),
            }
        }

        /// Creates a new instance of the RNG seeded via [`getrandom`] without
        /// unwrapping potential [`getrandom`] errors.
        ///
        /// [`getrandom`]: https://docs.rs/getrandom
        #[rune::function(free, path = $ty::try_from_os_rng)]
        #[cfg(feature = "os_rng")]
        fn try_from_os_rng() -> Result<$ty, Error> {
            match rand::SeedableRng::try_from_os_rng() {
                Ok(inner) => Ok($ty { inner }),
                Err(inner) => Err(Error { inner }),
            }
        }

        /// Create a new PRNG using the given seed.
        ///
        /// PRNG implementations are allowed to assume that bits in the seed are
        /// well distributed. That means usually that the number of one and zero
        /// bits are roughly equal, and values like 0, 1 and (size - 1) are
        /// unlikely. Note that many non-cryptographic PRNGs will show poor quality
        /// output if this is not adhered to. If you wish to seed from simple
        /// numbers, use [`seed_from_u64`] instead.
        ///
        /// All PRNG implementations should be reproducible unless otherwise noted:
        /// given a fixed `seed`, the same sequence of output should be produced on
        /// all runs, library versions and architectures (e.g. check endianness).
        /// Any "value-breaking" changes to the generator should require bumping at
        /// least the minor version and documentation of the change.
        ///
        /// It is not required that this function yield the same state as a
        /// reference implementation of the PRNG given equivalent seed; if necessary
        /// another constructor replicating behaviour from a reference
        /// implementation can be added.
        ///
        /// PRNG implementations should make sure `from_seed` never panics. In the
        /// case that some special values (like an all zero seed) are not viable
        /// seeds it is preferable to map these to alternative constant value(s),
        /// for example `0xBAD5EEDu32` or `0x0DDB1A5E5BAD5EEDu64` ("odd biases? bad
        /// seed"). This is assuming only a small number of values must be rejected.
        ///
        /// [`seed_from_u64`]: SmallRng::seed_from_u64
        #[rune::function(free, path = $ty::from_seed)]
        fn from_seed(seed: [u8; 32]) -> $ty {
            $ty {
                inner: rand::SeedableRng::from_seed(seed),
            }
        }

        /// Create a new PRNG using a `u64` seed.
        ///
        /// This is a convenience-wrapper around `from_seed` to allow construction
        /// of any `SeedableRng` from a simple `u64` value. It is designed such that
        /// low Hamming Weight numbers like 0 and 1 can be used and should still
        /// result in good, independent seeds to the PRNG which is returned.
        ///
        /// This **is not suitable for cryptography**, as should be clear given that
        /// the input size is only 64 bits.
        ///
        /// Implementations for PRNGs *may* provide their own implementations of
        /// this function, but the default implementation should be good enough for
        /// all purposes. *Changing* the implementation of this function should be
        /// considered a value-breaking change.
        #[rune::function(free, path = $ty::seed_from_u64)]
        fn seed_from_u64(state: u64) -> $ty {
            $ty {
                inner: rand::SeedableRng::seed_from_u64(state),
            }
        }
    }};
}

#[cfg(any(feature = "small_rng", feature = "std_rng", feature = "thread_rng"))]
macro_rules! random {
    ($m:ident, $ty:ty, $example:expr, $(($name:ident, $out:ty)),* $(,)?) => {
        $(
            #[doc = concat!(" Return a random `", stringify!($out), "` value via a standard uniform distribution.")]
            ///
            /// # Example
            ///
            /// ```rune
            #[doc = concat!(" use rand::", stringify!($ty), ";")]
            ///
            #[doc = concat!(" let rng = ", $example, ";")]
            #[doc = concat!(" let x = rng.random::<", stringify!($out), ">();")]
            /// println!("{x}");
            /// ```
            #[rune::function(instance, path = random<$out>)]
            fn $name(this: &mut $ty) -> $out {
                rand::Rng::random(&mut this.inner)
            }

            $m.function_meta($name)?;
        )*
    }
}

#[cfg(any(feature = "small_rng", feature = "std_rng", feature = "thread_rng"))]
macro_rules! random_ranges {
    ($m:ident, $ty:ty, $example:expr, $(($name:ident, $out:ty, $as:path, $range:expr)),* $(,)?) => {
        $(
            {
                use rune::runtime::{Range, RangeInclusive, TypeHash, Value, VmResult};
                use rune::vm_try;

                #[doc = concat!(" Return a random `", stringify!($out), "` value via a standard uniform constrained with a range.")]
                ///
                /// # Example
                ///
                /// ```rune
                #[doc = concat!(" use rand::", stringify!($ty), ";")]
                ///
                #[doc = concat!(" let rng = ", $example, ";")]
                #[doc = concat!(" let x = rng.random_range::<", stringify!($out), ">(", stringify!($range), ");")]
                /// println!("{x}");
                /// ```
                #[rune::function(instance, path = random_range<$out>)]
                fn $name(this: &mut $ty, range: Value) -> VmResult<$out> {
                    let value = match range.as_any() {
                        Some(value) => match value.type_hash() {
                            RangeInclusive::HASH => {
                                let range = vm_try!(value.borrow_ref::<RangeInclusive>());
                                let start = vm_try!($as(&range.start));
                                let end = vm_try!($as(&range.end));
                                rand::Rng::random_range(&mut this.inner, start..=end)
                            }
                            Range::HASH => {
                                let range = vm_try!(value.borrow_ref::<Range>());
                                let start = vm_try!($as(&range.start));
                                let end = vm_try!($as(&range.end));
                                rand::Rng::random_range(&mut this.inner, start..end)
                            }
                            _ => {
                                return VmResult::panic("unsupported range");
                            }
                        },
                        _ => {
                            return VmResult::panic("unsupported range");
                        }
                    };

                    VmResult::Ok(value)
                }

                $m.function_meta($name)?;
            }
        )*
    }
}
