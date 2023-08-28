use crate::runtime::vm::CallResult;
use crate::runtime::{
    GuardedArgs, Protocol, Stack, UnitFn, Value, Vm, VmError, VmErrorKind, VmResult,
};
use crate::Hash;

/// Trait used for integrating an instance function call.
pub(crate) trait ProtocolCaller: Sized {
    /// Call the given protocol function.
    fn call_protocol_fn<A>(
        &mut self,
        protocol: Protocol,
        target: Value,
        args: A,
    ) -> VmResult<Value>
    where
        A: GuardedArgs;

    /// Call the given protocol function.
    fn try_call_protocol_fn<A>(
        &mut self,
        protocol: Protocol,
        target: Value,
        args: A,
    ) -> VmResult<CallResult<Value>>
    where
        A: GuardedArgs,
        Self: Sized,
    {
        VmResult::Ok(CallResult::Ok(vm_try!(
            self.call_protocol_fn(protocol, target, args)
        )))
    }
}

/// Use the global environment caller.
///
/// This allocates its own stack and virtual machine for the call.
pub(crate) struct EnvProtocolCaller;

impl ProtocolCaller for EnvProtocolCaller {
    fn call_protocol_fn<A>(&mut self, protocol: Protocol, target: Value, args: A) -> VmResult<Value>
    where
        A: GuardedArgs,
    {
        return crate::runtime::env::with(|context, unit| {
            let count = args.count() + 1;
            let hash = Hash::associated_function(vm_try!(target.type_hash()), protocol.hash);

            if let Some(UnitFn::Offset {
                offset,
                args: expected,
                call,
            }) = unit.function(hash)
            {
                vm_try!(check_args(count, expected));

                let mut stack = vm_try!(Stack::with_capacity(count));
                vm_try!(stack.push(target));

                // Safety: We hold onto the guard until the vm has completed.
                let _guard = unsafe { vm_try!(args.unsafe_into_stack(&mut stack)) };

                let mut vm = Vm::with_stack(context.clone(), unit.clone(), stack);
                vm.set_ip(offset);
                return call.call_with_vm(vm);
            }

            let Some(handler) = context.function(hash) else {
                return VmResult::err(VmErrorKind::MissingInstanceFunction {
                    hash,
                    instance: vm_try!(target.type_info()),
                });
            };

            let mut stack = vm_try!(Stack::with_capacity(count));
            vm_try!(stack.push(target));

            // Safety: We hold onto the guard until the vm has completed.
            let _guard = unsafe { vm_try!(args.unsafe_into_stack(&mut stack)) };

            vm_try!(handler(&mut stack, count));
            VmResult::Ok(vm_try!(stack.pop()))
        });

        /// Check that arguments matches expected or raise the appropriate error.
        fn check_args(args: usize, expected: usize) -> Result<(), VmError> {
            if args != expected {
                return Err(VmError::from(VmErrorKind::BadArgumentCount {
                    actual: args,
                    expected,
                }));
            }

            Ok(())
        }
    }
}

impl ProtocolCaller for Vm {
    fn call_protocol_fn<A>(&mut self, protocol: Protocol, target: Value, args: A) -> VmResult<Value>
    where
        A: GuardedArgs,
    {
        if let CallResult::Unsupported(..) = vm_try!(self.call_instance_fn(target, protocol, args))
        {
            return VmResult::err(VmErrorKind::MissingFunction {
                hash: protocol.hash,
            });
        }

        VmResult::Ok(vm_try!(self.stack_mut().pop()))
    }
}
