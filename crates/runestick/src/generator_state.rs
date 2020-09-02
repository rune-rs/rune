use crate::{
    FromValue, OwnedMut, OwnedRef, RawOwnedMut, RawOwnedRef, Shared, UnsafeFromValue, Value,
    ValueError,
};

value_types!(crate::GENERATOR_STATE_TYPE, GeneratorState => GeneratorState, &GeneratorState, &mut GeneratorState);

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
    fn from_value(value: Value) -> Result<Self, ValueError> {
        Ok(value.into_generator_state()?)
    }
}

impl FromValue for GeneratorState {
    fn from_value(value: Value) -> Result<Self, ValueError> {
        let state = value.into_generator_state()?;
        Ok(state.take()?)
    }
}

impl UnsafeFromValue for &GeneratorState {
    type Output = *const GeneratorState;
    type Guard = RawOwnedRef;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), ValueError> {
        let state = value.into_generator_state()?;
        let (state, guard) = OwnedRef::into_raw(state.owned_ref()?);
        Ok((state, guard))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}

impl UnsafeFromValue for &mut GeneratorState {
    type Output = *mut GeneratorState;
    type Guard = RawOwnedMut;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), ValueError> {
        let state = value.into_generator_state()?;
        Ok(OwnedMut::into_raw(state.owned_mut()?))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &mut *output
    }
}
