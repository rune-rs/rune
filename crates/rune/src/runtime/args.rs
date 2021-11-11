use crate::runtime::{Stack, ToValue, Value, VmError};

/// Trait for converting arguments onto the stack.
pub trait Args {
    /// Encode arguments onto a stack.
    fn into_stack(self, stack: &mut Stack) -> Result<(), VmError>;

    /// Convert arguments into a vector.
    fn into_vec(self) -> Result<Vec<Value>, VmError>;

    /// The number of arguments.
    fn count(&self) -> usize;
}

macro_rules! impl_into_args {
    () => {
        impl_into_args!{@impl 0,}
    };

    ({$ty:ident, $value:ident, $count:expr}, $({$l_ty:ident, $l_value:ident, $l_count:expr},)*) => {
        impl_into_args!{@impl $count, {$ty, $value, $count}, $({$l_ty, $l_value, $l_count},)*}
        impl_into_args!{$({$l_ty, $l_value, $l_count},)*}
    };

    (@impl $count:expr, $({$ty:ident, $value:ident, $ignore_count:expr},)*) => {
        impl<$($ty,)*> Args for ($($ty,)*)
        where
            $($ty: ToValue,)*
        {
            #[allow(unused)]
            fn into_stack(self, stack: &mut Stack) -> Result<(), VmError> {
                let ($($value,)*) = self;
                $(stack.push($value.to_value()?);)*
                Ok(())
            }

            #[allow(unused)]
            fn into_vec(self) -> Result<Vec<Value>, VmError> {
                let ($($value,)*) = self;
                $(let $value = <$ty>::to_value($value)?;)*
                Ok(vec![$($value,)*])
            }

            fn count(&self) -> usize {
                $count
            }
        }
    };
}

repeat_macro!(impl_into_args);

impl Args for Vec<Value> {
    fn into_stack(self, stack: &mut Stack) -> Result<(), VmError> {
        for value in self {
            stack.push(value);
        }
        Ok(())
    }

    fn into_vec(self) -> Result<Vec<Value>, VmError> {
        Ok(self)
    }

    fn count(&self) -> usize {
        self.len()
    }
}
