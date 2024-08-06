use core::fmt;

/// Used to ergonomically display an address.
pub(super) struct DisplayNamed<T> {
    value: T,
    name: Option<&'static str>,
}

impl<T> DisplayNamed<T> {
    #[inline]
    pub(super) const fn new(value: T, name: Option<&'static str>) -> Self {
        Self { value, name }
    }
}

impl<T> fmt::Display for DisplayNamed<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.name {
            Some(name) => write!(f, "{} ({})", self.value, name),
            None => self.value.fmt(f),
        }
    }
}
