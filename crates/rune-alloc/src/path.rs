use core::borrow::Borrow;
use core::clone::Clone;
use core::convert::AsRef;
use core::fmt;
use core::ops::Deref;

use rust_alloc::boxed::Box;

#[derive(Debug)]
#[repr(transparent)]
pub struct Path {
    inner: [u8],
}

impl Path {
    #[inline]
    pub fn new<P>(path: &P) -> &Path
    where
        P: ?Sized + AsRef<Path>,
    {
        unsafe { &*(path.as_ref() as *const _ as *const Path) }
    }

    pub fn display(&self) -> Display<'_> {
        Display { _path: self }
    }

    pub fn join<P>(&self, _: P) -> PathBuf
    where
        P: AsRef<Path>,
    {
        PathBuf
    }

    pub fn with_extension<S>(&self, _: S) -> PathBuf
    where
        S: AsRef<str>,
    {
        PathBuf
    }

    pub fn is_file(&self) -> bool {
        false
    }

    pub fn to_path_buf(&self) -> PathBuf {
        PathBuf
    }
}

impl AsRef<Path> for str {
    #[inline]
    fn as_ref(&self) -> &Path {
        Path::new(self)
    }
}

pub struct Display<'a> {
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

#[derive(Debug, Clone)]
pub struct PathBuf;

impl PathBuf {
    pub fn push<P>(&mut self, _: P)
    where
        P: AsRef<Path>,
    {
        // Do nothing
    }

    pub fn pop(&mut self) -> bool {
        true
    }

    pub fn into_boxed_path(self) -> Box<Path> {
        let ptr = Box::into_raw(Box::<[u8]>::default()) as *mut Path;
        // SAFETY: Layout of Path and [u8] is the same.
        unsafe { Box::from_raw(ptr) }
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

impl Clone for Box<Path> {
    fn clone(&self) -> Self {
        self.to_path_buf().into_boxed_path()
    }
}

impl From<&Path> for Box<Path> {
    fn from(path: &Path) -> Self {
        path.to_path_buf().into_boxed_path()
    }
}
