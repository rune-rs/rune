use core::ops;

use crate as rune;
use crate::alloc::clone::TryClone;
use crate::alloc::fmt::TryWrite;
use crate::Any;

use super::{
    EnvProtocolCaller, Formatter, FromValue, ProtocolCaller, RuntimeError, ToValue, Value, VmResult,
};

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
#[derive(Debug, Clone, TryClone, Any)]
#[try_clone(crate)]
#[rune(static_type = CONTROL_FLOW)]
pub enum ControlFlow {
    /// Move on to the next phase of the operation as normal.
    #[rune(constructor)]
    Continue(#[rune(get, set)] Value),
    /// Exit the operation without running subsequent phases.
    #[rune(constructor)]
    Break(#[rune(get, set)] Value),
}

impl ControlFlow {
    /// Test two control flows for partial equality.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::{partial_eq, ControlFlow};
    ///
    /// assert_eq! {
    ///     partial_eq(ControlFlow::Continue(true), ControlFlow::Continue(true)),
    ///     true
    /// };
    /// assert_eq! {
    ///     partial_eq(ControlFlow::Continue(true), ControlFlow::Break(false)),
    ///     false
    /// };
    /// assert_eq! {
    ///     partial_eq(ControlFlow::Break(false), ControlFlow::Continue(true)),
    ///     false
    /// };
    /// ```
    #[rune::function(keep, protocol = PARTIAL_EQ)]
    pub(crate) fn partial_eq(&self, other: &Self) -> VmResult<bool> {
        Self::partial_eq_with(self, other, &mut EnvProtocolCaller)
    }

    pub(crate) fn partial_eq_with(
        &self,
        other: &Self,
        caller: &mut dyn ProtocolCaller,
    ) -> VmResult<bool> {
        match (self, other) {
            (ControlFlow::Continue(a), ControlFlow::Continue(b)) => {
                Value::partial_eq_with(a, b, caller)
            }
            (ControlFlow::Break(a), ControlFlow::Break(b)) => Value::partial_eq_with(a, b, caller),
            _ => VmResult::Ok(false),
        }
    }

    /// Test two control flows for total equality.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::{eq, ControlFlow};
    ///
    /// assert_eq! {
    ///     eq(ControlFlow::Continue(true), ControlFlow::Continue(true)),
    ///     true
    /// };
    /// assert_eq! {
    ///     eq(ControlFlow::Continue(true), ControlFlow::Break(false)),
    ///     false
    /// };
    /// assert_eq! {
    ///     eq(ControlFlow::Break(false), ControlFlow::Continue(true)),
    ///     false
    /// };
    /// ```
    #[rune::function(keep, protocol = EQ)]
    pub(crate) fn eq(&self, other: &ControlFlow) -> VmResult<bool> {
        self.eq_with(other, &mut EnvProtocolCaller)
    }

    pub(crate) fn eq_with(
        &self,
        other: &ControlFlow,
        caller: &mut dyn ProtocolCaller,
    ) -> VmResult<bool> {
        match (self, other) {
            (ControlFlow::Continue(a), ControlFlow::Continue(b)) => Value::eq_with(a, b, caller),
            (ControlFlow::Break(a), ControlFlow::Break(b)) => Value::eq_with(a, b, caller),
            _ => VmResult::Ok(false),
        }
    }

    /// Debug print the control flow.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::ControlFlow;
    ///
    /// let string = format!("{:?}", ControlFlow::Continue(true));
    /// ```
    #[rune::function(keep, protocol = STRING_DEBUG)]
    pub(crate) fn string_debug(&self, f: &mut Formatter) -> VmResult<()> {
        Self::string_debug_with(self, f, &mut EnvProtocolCaller)
    }

    pub(crate) fn string_debug_with(
        &self,
        f: &mut Formatter,
        caller: &mut dyn ProtocolCaller,
    ) -> VmResult<()> {
        match self {
            ControlFlow::Continue(value) => {
                vm_try!(vm_write!(f, "Continue("));
                vm_try!(Value::string_debug_with(value, f, caller));
                vm_try!(vm_write!(f, ")"));
            }
            ControlFlow::Break(value) => {
                vm_try!(vm_write!(f, "Break("));
                vm_try!(Value::string_debug_with(value, f, caller));
                vm_try!(vm_write!(f, ")"));
            }
        }

        VmResult::Ok(())
    }

    /// Clone the control flow.
    ///
    /// # Examples
    ///
    /// ```rune
    /// use std::ops::ControlFlow;
    ///
    /// let flow = ControlFlow::Continue("Hello World");
    /// let flow2 = flow.clone();
    ///
    /// assert_eq!(flow, flow2);
    /// ```
    #[rune::function(keep, protocol = CLONE)]
    pub(crate) fn clone(&self) -> VmResult<Self> {
        VmResult::Ok(vm_try!(self.try_clone()))
    }
}

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
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        Ok(match &*value.borrow_ref::<ControlFlow>()? {
            ControlFlow::Continue(value) => {
                ops::ControlFlow::Continue(C::from_value(value.clone())?)
            }
            ControlFlow::Break(value) => ops::ControlFlow::Break(B::from_value(value.clone())?),
        })
    }
}
