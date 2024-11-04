use crate::runtime::vm::{CallResult, CallResultOnly, Isolated};
use crate::runtime::{
    DynArgs, Protocol, Stack, UnitFn, Value, Vm, VmError, VmErrorKind, VmExecution, VmResult,
};
use crate::Hash;

/// Trait used for integrating an instance function call.
pub(crate) trait ProtocolCaller: 'static {
    /// Call the given protocol function.
    fn call_protocol_fn(
        &mut self,
        protocol: Protocol,
        target: Value,
        args: &mut dyn DynArgs,
    ) -> VmResult<Value> {
        match vm_try!(self.try_call_protocol_fn(protocol, target, args)) {
            CallResultOnly::Ok(value) => VmResult::Ok(value),
            CallResultOnly::Unsupported(value) => {
                VmResult::err(VmErrorKind::MissingProtocolFunction {
                    protocol,
                    instance: value.type_info(),
                })
            }
        }
    }

    /// Call the given protocol function.
    fn try_call_protocol_fn(
        &mut self,
        protocol: Protocol,
        target: Value,
        args: &mut dyn DynArgs,
    ) -> VmResult<CallResultOnly<Value>>;
}

/// Use the global environment caller.
///
/// This allocates its own stack and virtual machine for the call.
pub(crate) struct EnvProtocolCaller;

impl ProtocolCaller for EnvProtocolCaller {
    fn try_call_protocol_fn(
        &mut self,
        protocol: Protocol,
        target: Value,
        args: &mut dyn DynArgs,
    ) -> VmResult<CallResultOnly<Value>> {
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
            }) = unit.function(hash)
            {
                vm_try!(check_args(count, expected));

                let mut stack = vm_try!(Stack::with_capacity(count));
                vm_try!(stack.push(target));
                vm_try!(args.push_to_stack(&mut stack));
                let mut vm = Vm::with_stack(context.clone(), unit.clone(), stack);
                vm.set_ip(offset);
                return VmResult::Ok(CallResultOnly::Ok(vm_try!(call.call_with_vm(vm))));
            }

            if let Some(handler) = context.function(hash) {
                let mut stack = vm_try!(Stack::with_capacity(count));
                let addr = stack.addr();
                vm_try!(stack.push(target));
                vm_try!(args.push_to_stack(&mut stack));
                vm_try!(handler(&mut stack, addr, count, addr.output()));
                let value = vm_try!(stack.at(addr)).clone();
                return VmResult::Ok(CallResultOnly::Ok(value));
            }

            VmResult::Ok(CallResultOnly::Unsupported(target))
        })
    }
}

impl ProtocolCaller for Vm {
    fn try_call_protocol_fn(
        &mut self,
        protocol: Protocol,
        target: Value,
        args: &mut dyn DynArgs,
    ) -> VmResult<CallResultOnly<Value>> {
        let addr = self.stack().addr();
        vm_try!(self.stack_mut().push(()));

        match vm_try!(self.call_instance_fn(
            Isolated::Isolated,
            target,
            protocol,
            args,
            addr.output()
        )) {
            CallResult::Unsupported(value) => VmResult::Ok(CallResultOnly::Unsupported(value)),
            CallResult::Ok(()) => {
                let value = vm_try!(self.stack().at(addr)).clone();
                self.stack_mut().truncate(addr);
                VmResult::Ok(CallResultOnly::Ok(value))
            }
            CallResult::Frame => {
                let mut execution = VmExecution::new(self);
                let value = vm_try!(execution.complete());
                execution.vm_mut().stack_mut().truncate(addr);
                VmResult::Ok(CallResultOnly::Ok(value))
            }
        }
    }
}
