use alloc::boxed::Box;

pub(crate) trait Error {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl<E> Error for Box<E>
where
    E: Error,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        (**self).source()
    }
}
