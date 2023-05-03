use core::clone::Clone;
use core::convert::AsRef;
use core::fmt;
use core::ops::Deref;

use alloc::borrow::Borrow;
use alloc::borrow::ToOwned;
use alloc::boxed::Box;

#[derive(Debug)]
#[repr(transparent)]
pub struct Path {
    inner: [u8],
}

impl Path {
    #[inline]
    pub(crate) fn new<P>(path: &P) -> &Path
    where
        P: ?Sized + AsRef<Path>,
    {
        unsafe { &*(path.as_ref() as *const _ as *const Path) }
    }

    pub(crate) fn display(&self) -> Display<'_> {
        Display { _path: self }
    }

    pub(crate) fn join<P>(&self, _: P) -> PathBuf
    where
        P: AsRef<Path>,
    {
        PathBuf
    }

    pub(crate) fn with_extension<S>(&self, _: S) -> PathBuf
    where
        S: AsRef<str>,
    {
        PathBuf
    }

    pub(crate) fn is_file(&self) -> bool {
        false
    }
}

impl AsRef<Path> for str {
    #[inline]
    fn as_ref(&self) -> &Path {
        Path::new(self)
    }
}

pub(crate) struct Display<'a> {
    _path: &'a Path,
}

impl fmt::Display for Display<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "?")
    }
}

impl AsRef<Path> for Path {
    #[inline]
    fn as_ref(&self) -> &Path {
        self
    }
}

impl Clone for Box<Path> {
    fn clone(&self) -> Box<Path> {
        todo!()
    }
}

impl From<&Path> for Box<Path> {
    #[inline]
    fn from(p: &Path) -> Self {
        let rw = Box::into_raw(Box::<[u8]>::from(p.inner.to_vec())) as *mut Path;
        unsafe { Box::from_raw(rw) }
    }
}

impl ToOwned for Path {
    type Owned = PathBuf;

    #[inline]
    fn to_owned(&self) -> Self::Owned {
        PathBuf
    }
}

#[derive(Debug, Clone)]
pub struct PathBuf;

impl PathBuf {
    pub(crate) fn push<P>(&mut self, _: P)
    where
        P: AsRef<Path>,
    {
        // Do nothing
    }

    pub(crate) fn pop(&mut self) -> bool {
        true
    }
}

impl AsRef<Path> for PathBuf {
    #[inline]
    fn as_ref(&self) -> &Path {
        self
    }
}

impl From<&Path> for PathBuf {
    #[inline]
    fn from(_: &Path) -> Self {
        Self
    }
}

impl Borrow<Path> for PathBuf {
    #[inline]
    fn borrow(&self) -> &Path {
        self
    }
}

impl Deref for PathBuf {
    type Target = Path;

    #[inline]
    fn deref(&self) -> &Self::Target {
        Path::new("")
    }
}
