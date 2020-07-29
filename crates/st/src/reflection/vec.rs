use crate::reflection::{FromValue, ToValue};
use crate::value::ValuePtr;
use crate::vm::{StackError, Vm};

impl<T> FromValue for Vec<T>
where
    T: FromValue,
{
    fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, StackError> {
        let slot = value.into_array()?;
        let array = vm.array_take(slot)?;

        let mut output = Vec::with_capacity(array.len());

        for value in array.iter().copied() {
            output.push(T::from_value(value, vm)?);
        }

        Ok(output)
    }
}

impl<T> ToValue for Vec<T>
where
    T: ToValue,
{
    fn to_value(self, vm: &mut Vm) -> Result<ValuePtr, StackError> {
        let mut array = Vec::with_capacity(self.len());

        for value in self {
            array.push(value.to_value(vm)?);
        }

        Ok(vm.allocate_array(array))
    }
}
