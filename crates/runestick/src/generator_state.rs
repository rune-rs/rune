use crate::{FromValue, Mut, RawMut, RawRef, Ref, Shared, UnsafeFromValue, Value, VmError};

/// The state of a generator.
#[derive(Debug)]
pub enum GeneratorState {
    /// The generator yielded.
    Yielded(Value),
    /// The generator completed.
    Complete(Value),
}

impl GeneratorState {
    /// Test if the state is yielded.
    pub fn is_yielded(&self) -> bool {
        matches!(self, Self::Yielded(..))
    }

    /// Test if the state is complete.
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete(..))
    }
}

impl FromValue for Shared<GeneratorState> {
    fn from_value(value: Value) -> Result<Self, VmError> {
        value.into_generator_state()
    }
}

impl FromValue for GeneratorState {
    fn from_value(value: Value) -> Result<Self, VmError> {
        let state = value.into_generator_state()?;
        Ok(state.take()?)
    }
}

impl UnsafeFromValue for &GeneratorState {
    type Output = *const GeneratorState;
    type Guard = RawRef;

    fn from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let state = value.into_generator_state()?;
        let (state, guard) = Ref::into_raw(state.into_ref()?);
        Ok((state, guard))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &*output
    }
}

impl UnsafeFromValue for &mut GeneratorState {
    type Output = *mut GeneratorState;
    type Guard = RawMut;

    fn from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let state = value.into_generator_state()?;
        Ok(Mut::into_raw(state.into_mut()?))
    }

    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &mut *output
    }
}
