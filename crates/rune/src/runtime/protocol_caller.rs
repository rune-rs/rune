use crate::runtime::vm::{CallResult, CallResultOnly, Isolated};
use crate::runtime::{
    DynArgs, Protocol, Stack, UnitFn, Value, Vm, VmError, VmErrorKind, VmExecution,
};
use crate::Hash;

/// Trait used for integrating an instance function call.
pub(crate) trait ProtocolCaller: 'static {
    /// Call the given protocol function.
    fn call_protocol_fn(
        &mut self,
        protocol: &'static Protocol,
        target: Value,
        args: &mut dyn DynArgs,
    ) -> Result<Value, VmError> {
        match self.try_call_protocol_fn(protocol, target, args)? {
            CallResultOnly::Ok(value) => Ok(value),
            CallResultOnly::Unsupported(value) => {
                Err(VmError::new(VmErrorKind::MissingProtocolFunction {
                    protocol,
                    instance: value.type_info(),
                }))
            }
        }
    }

    /// Call the given protocol function.
    fn try_call_protocol_fn(
        &mut self,
        protocol: &'static Protocol,
        target: Value,
        args: &mut dyn DynArgs,
    ) -> Result<CallResultOnly<Value>, VmError>;
}

/// Use the global environment caller.
///
/// This allocates its own stack and virtual machine for the call.
pub(crate) struct EnvProtocolCaller;

impl ProtocolCaller for EnvProtocolCaller {
    fn try_call_protocol_fn(
        &mut self,
        protocol: &Protocol,
        target: Value,
        args: &mut dyn DynArgs,
    ) -> Result<CallResultOnly<Value>, VmError> {
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

        crate::runtime::env::shared(|context, unit| {
            let count = args.count() + 1;
            let hash = Hash::associated_function(target.type_hash(), protocol.hash);

            if let Some(UnitFn::Offset {
                offset,
                args: expected,
                call,
                ..
            }) = unit.function(&hash)
            {
                check_args(count, *expected)?;

                let mut stack = Stack::with_capacity(count)?;
                stack.push(target)?;
                args.push_to_stack(&mut stack)?;
                let mut vm = Vm::with_stack(context.clone(), unit.clone(), stack);
                vm.set_ip(*offset);
                return Ok(CallResultOnly::Ok(call.call_with_vm(vm)?));
            }

            if let Some(handler) = context.function(&hash) {
                let mut stack = Stack::with_capacity(count)?;
                let addr = stack.addr();
                stack.push(target)?;
                args.push_to_stack(&mut stack)?;
                handler.call(&mut stack, addr, count, addr.output())?;
                let value = stack.at(addr).clone();
                return Ok(CallResultOnly::Ok(value));
            }

            Ok(CallResultOnly::Unsupported(target))
        })
    }
}

impl ProtocolCaller for Vm {
    #[inline]
    fn try_call_protocol_fn(
        &mut self,
        protocol: &'static Protocol,
        target: Value,
        args: &mut dyn DynArgs,
    ) -> Result<CallResultOnly<Value>, VmError> {
        let addr = self.stack().addr();
        self.stack_mut().push(())?;

        let result = match self.call_instance_fn(
            Isolated::Isolated,
            target,
            protocol,
            args,
            addr.output(),
        )? {
            CallResult::Unsupported(value) => CallResultOnly::Unsupported(value),
            CallResult::Ok(()) => {
                let value = self.stack().at(addr).clone();
                CallResultOnly::Ok(value)
            }
            CallResult::Frame => CallResultOnly::Ok(VmExecution::new(&mut *self).complete()?),
        };

        self.stack_mut().truncate(addr);
        Ok(result)
    }
}
