use crate::Value;
use std::fmt;
use std::ops;

value_types!(crate::TUPLE_TYPE, Tuple => Tuple);

/// Struct representing an anonymous tuple.
#[derive(Clone)]
#[repr(transparent)]
pub struct Tuple {
    inner: Box<[Value]>,
}

impl Tuple {
    /// Convert into inner.
    pub fn into_inner(self) -> Box<[Value]> {
        self.inner
    }
}

impl fmt::Debug for Tuple {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")?;

        let mut it = self.inner.iter();
        let last = it.next_back();

        for el in it {
            write!(f, "{:?}, ", el)?;
        }

        if let Some(last) = last {
            write!(f, "{:?}", last)?;
        }

        write!(f, ")")?;
        Ok(())
    }
}

impl ops::Deref for Tuple {
    type Target = [Value];

    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl ops::DerefMut for Tuple {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.inner
    }
}

impl From<Vec<Value>> for Tuple {
    fn from(vec: Vec<Value>) -> Self {
        Self {
            inner: vec.into_boxed_slice(),
        }
    }
}

impl From<Box<[Value]>> for Tuple {
    fn from(inner: Box<[Value]>) -> Self {
        Self { inner }
    }
}
