use core::ops;

use crate as rune;
use crate::alloc::fmt::TryWrite;
use crate::runtime::{Formatter, FromValue, ProtocolCaller, ToValue, Value, VmResult};
use crate::Any;

/// Used to tell an operation whether it should exit early or go on as usual.
///
/// This acts as the basis of the [`TRY`] protocol in Rune.
///
/// [`TRY`]: crate::runtime::Protocol::TRY
///
/// # Examples
///
/// ```rune
/// use std::ops::ControlFlow;
///
/// let c = ControlFlow::Continue(42);
/// assert_eq!(c.0, 42);
/// assert_eq!(c, ControlFlow::Continue(42));
/// ```
#[derive(Debug, Clone, Any)]
#[rune(builtin, static_type = CONTROL_FLOW_TYPE)]
pub enum ControlFlow {
    /// Move on to the next phase of the operation as normal.
    #[rune(constructor)]
    Continue(#[rune(get, set)] Value),
    /// Exit the operation without running subsequent phases.
    #[rune(constructor)]
    Break(#[rune(get, set)] Value),
}

impl ControlFlow {
    pub(crate) fn string_debug_with(
        &self,
        f: &mut Formatter,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<()> {
        match self {
            ControlFlow::Continue(value) => {
                vm_write!(f, "Continue(");
                vm_try!(Value::string_debug_with(value, f, caller));
                vm_write!(f, ")");
            }
            ControlFlow::Break(value) => {
                vm_write!(f, "Break(");
                vm_try!(Value::string_debug_with(value, f, caller));
                vm_write!(f, ")");
            }
        }

        VmResult::Ok(())
    }

    pub(crate) fn partial_eq_with(
        &self,
        other: &Self,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<bool> {
        match (self, other) {
            (ControlFlow::Continue(a), ControlFlow::Continue(b)) => {
                Value::partial_eq_with(a, b, caller)
            }
            (ControlFlow::Break(a), ControlFlow::Break(b)) => Value::partial_eq_with(a, b, caller),
            _ => VmResult::Ok(false),
        }
    }

    pub(crate) fn eq_with(
        &self,
        other: &ControlFlow,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<bool> {
        match (self, other) {
            (ControlFlow::Continue(a), ControlFlow::Continue(b)) => Value::eq_with(a, b, caller),
            (ControlFlow::Break(a), ControlFlow::Break(b)) => Value::eq_with(a, b, caller),
            _ => VmResult::Ok(false),
        }
    }
}

from_value!(ControlFlow, into_control_flow);

impl<B, C> ToValue for ops::ControlFlow<B, C>
where
    B: ToValue,
    C: ToValue,
{
    #[inline]
    fn to_value(self) -> VmResult<Value> {
        let value = match self {
            ops::ControlFlow::Continue(value) => {
                ControlFlow::Continue(vm_try!(ToValue::to_value(value)))
            }
            ops::ControlFlow::Break(value) => ControlFlow::Break(vm_try!(ToValue::to_value(value))),
        };

        VmResult::Ok(vm_try!(Value::try_from(value)))
    }
}

impl<B, C> FromValue for ops::ControlFlow<B, C>
where
    B: FromValue,
    C: FromValue,
{
    #[inline]
    fn from_value(value: Value) -> VmResult<Self> {
        let value = vm_try!(value.into_control_flow());

        VmResult::Ok(match vm_try!(value.take()) {
            ControlFlow::Continue(value) => {
                ops::ControlFlow::Continue(vm_try!(C::from_value(value)))
            }
            ControlFlow::Break(value) => ops::ControlFlow::Break(vm_try!(B::from_value(value))),
        })
    }
}
