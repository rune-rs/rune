use crate::runtime::{Stack, UnsafeFromValue, VmErrorKind, VmResult};

pub type ArgumentPacket<F, M> = (
    <F as RawFunction<M>>::RawArgumentFirst,
    <F as RawFunction<M>>::RawArgumentSuffix,
    <F as RawFunction<M>>::RawGuard,
);

pub trait RawFunction<Marker>: 'static + Send + Sync {
    /// An object guarding the lifetime of the arguments.
    #[doc(hidden)]
    type RawGuard;

    /// The first argument type.
    #[doc(hidden)]
    type RawArgumentFirst;

    /// The argument types after the first.
    #[doc(hidden)]
    type RawArgumentSuffix;

    /// A tuple representing the argument type.
    #[doc(hidden)]
    type RawArguments;

    /// The raw return type of the function.
    #[doc(hidden)]
    type RawReturn;

    /// Get the number of arguments.
    #[doc(hidden)]
    fn raw_args() -> usize;

    /// Gets the argument packet.
    /// Safety: The guard must live as long as the arguments are in use.
    unsafe fn raw_get_args(
        &self,
        stack: &mut Stack,
        args: usize,
    ) -> VmResult<ArgumentPacket<Self, Marker>>;

    /// Safety: We hold a reference to the stack, so we can
    /// guarantee that it won't be modified.
    ///
    /// The scope is also necessary, since we mutably access `stack`
    /// when we return below.
    #[must_use]
    #[doc(hidden)]
    unsafe fn raw_call_packet(
        &self,
        packet: ArgumentPacket<Self, Marker>,
    ) -> VmResult<(Self::RawReturn, Self::RawGuard)>;

    /// This can be cleaned up once the arguments are no longer in use.
    #[doc(hidden)]
    unsafe fn raw_drop_guard(guard: Self::RawGuard);
}

#[doc(hidden)]
pub struct NoFirstArg(());

impl<U, T> RawFunction<fn() -> U> for T
where
    T: 'static + Send + Sync + Fn() -> U,
{
    type RawArgumentFirst = NoFirstArg;
    type RawArgumentSuffix = ();
    type RawArguments = ();
    type RawReturn = U;
    type RawGuard = ();
    fn raw_args() -> usize {
        0
    }
    unsafe fn raw_get_args(
        &self,
        _stack: &mut Stack,
        args: usize,
    ) -> VmResult<ArgumentPacket<T, fn() -> U>> {
        vm_try!(check_args(0, args));
        VmResult::Ok((NoFirstArg(()), (), ()))
    }
    unsafe fn raw_call_packet(
        &self,
        (_, _, guard): (
            Self::RawArgumentFirst,
            Self::RawArgumentSuffix,
            Self::RawGuard,
        ),
    ) -> VmResult<(Self::RawReturn, Self::RawGuard)> {
        VmResult::Ok((self(), guard))
    }
    unsafe fn raw_drop_guard(_guard: Self::RawGuard) {}
}

macro_rules! impl_register {
  ($count:expr $(, $ty:ident $var:ident $num:expr)*) => {
      impl<U, T, First, $($ty),*> RawFunction<fn(First, $($ty,)*) -> U> for T
      where
          T: 'static + Send + Sync + Fn(First, $($ty,)*) -> U,
          First: UnsafeFromValue,
          $($ty: UnsafeFromValue,)*
      {
          type RawArgumentFirst = First;
          type RawArgumentSuffix = ($($ty,)*);
          type RawArguments = (First, $($ty,)*);
          type RawReturn = U;
          type RawGuard = (First::Guard, $($ty::Guard,)*);
          fn raw_args() -> usize {
              $count + 1
          }
          unsafe fn raw_get_args(
            &self,
            stack: &mut Stack,
            args: usize
          ) -> VmResult<ArgumentPacket<T, fn(First, $($ty,)*) -> U>> {
            vm_try!(check_args($count+1, args));
            let [first $(, $var)*] = vm_try!(stack.drain_vec($count+1));

            let first = vm_try!(First::from_value(first).with_error(|| VmErrorKind::BadArgument {
                arg: 0,
            }));

            $(
                let $var = vm_try!(<$ty>::from_value($var).with_error(|| VmErrorKind::BadArgument {
                    arg: 1 + $num,
                }));
            )*

            let guard = (first.1 $(, $var.1)*, );
            let suffix = ( $(<$ty>::unsafe_coerce($var.0),)* );
            let first = First::unsafe_coerce(first.0);

            VmResult::Ok((first, suffix, guard))
          }
          unsafe fn raw_call_packet(
              &self,
              packet: ArgumentPacket<T, fn(First, $($ty,)*) -> U>,
          ) -> VmResult<(Self::RawReturn, Self::RawGuard)> {
              let (first, ($($var,)*), guard) = packet;
              VmResult::Ok((self(first $(,$var)*), guard))
          }
          unsafe fn raw_drop_guard(guard: Self::RawGuard) {
            let (inst, $($var,)*) = guard;
              drop(inst);
              $(drop(($var));)*
          }
      }
  };
}
repeat_macro!(impl_register);

fn check_args(expected: usize, actual: usize) -> VmResult<()> {
    if actual == expected {
        VmResult::Ok(())
    } else {
        VmResult::err(VmErrorKind::BadArgumentCount { actual, expected })
    }
}
