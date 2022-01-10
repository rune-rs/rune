use crate::runtime::vm::CallResult;
use crate::runtime::{GuardedArgs, Protocol, Stack, UnitFn, Value, Vm, VmError, VmErrorKind};
use crate::Hash;

/// Trait used for integrating an instance function call.
pub(crate) trait ProtocolCaller {
    /// Call the given protocol function.
    fn call_protocol_fn<A>(
        self,
        protocol: Protocol,
        target: Value,
        args: A,
    ) -> Result<Value, VmError>
    where
        A: GuardedArgs;
}

/// Use the global environment caller.
///
/// This allocates its own stack and virtual machine for the call.
pub(crate) struct EnvProtocolCaller;

impl ProtocolCaller for EnvProtocolCaller {
    fn call_protocol_fn<A>(
        self,
        protocol: Protocol,
        target: Value,
        args: A,
    ) -> Result<Value, VmError>
    where
        A: GuardedArgs,
    {
        return crate::runtime::env::with(|context, unit| {
            let count = args.count() + 1;
            let hash = Hash::instance_function(target.type_hash()?, protocol.hash);

            if let Some(UnitFn::Offset {
                offset,
                args: expected,
                call,
            }) = unit.function(hash)
            {
                check_args(count, expected)?;

                let mut stack = Stack::with_capacity(count);
                stack.push(target);

                // Safety: We hold onto the guard until the vm has completed.
                let _guard = unsafe { args.unsafe_into_stack(&mut stack)? };

                let mut vm = Vm::with_stack(context.clone(), unit.clone(), stack);
                vm.set_ip(offset);
                return call.call_with_vm(vm);
            }

            let handler = match context.function(hash) {
                Some(handler) => handler,
                None => return Err(VmError::from(VmErrorKind::MissingFunction { hash })),
            };

            let mut stack = Stack::with_capacity(count);
            stack.push(target);

            // Safety: We hold onto the guard until the vm has completed.
            let _guard = unsafe { args.unsafe_into_stack(&mut stack)? };

            handler(&mut stack, count)?;
            Ok(stack.pop()?)
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

impl ProtocolCaller for &mut Vm {
    fn call_protocol_fn<A>(
        self,
        protocol: Protocol,
        target: Value,
        args: A,
    ) -> Result<Value, VmError>
    where
        A: GuardedArgs,
    {
        if let CallResult::Unsupported(..) = self.call_instance_fn(target, protocol, args)? {
            return Err(VmError::from(VmErrorKind::MissingFunction {
                hash: protocol.hash,
            }));
        }

        Ok(self.stack_mut().pop()?)
    }
}
