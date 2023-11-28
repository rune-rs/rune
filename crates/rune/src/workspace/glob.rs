#[cfg(test)]
mod tests;

use std::fmt;
use std::fs;
use std::io;
use std::mem;
use std::path::{Path, PathBuf};

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::{self, Box, Vec, VecDeque};

use relative_path::RelativePath;

/// Errors raised during glob expansion.
#[derive(Debug)]
pub enum GlobError {
    Io(io::Error),
    Alloc(alloc::Error),
}

impl From<io::Error> for GlobError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<alloc::Error> for GlobError {
    fn from(error: alloc::Error) -> Self {
        Self::Alloc(error)
    }
}

impl fmt::Display for GlobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GlobError::Io(error) => error.fmt(f),
            GlobError::Alloc(error) => error.fmt(f),
        }
    }
}

cfg_std! {
    impl std::error::Error for GlobError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                GlobError::Io(error) => Some(error),
                GlobError::Alloc(error) => Some(error),
            }
        }
    }
}

/// A compiled glob expression.
pub struct Glob<'a> {
    root: &'a Path,
    components: Vec<Component<'a>>,
}

impl<'a> Glob<'a> {
    /// Construct a new glob pattern.
    pub fn new<R, P>(root: &'a R, pattern: &'a P) -> alloc::Result<Self>
    where
        R: ?Sized + AsRef<Path>,
        P: ?Sized + AsRef<RelativePath>,
    {
        let components = compile_pattern(pattern)?;

        Ok(Self {
            root: root.as_ref(),
            components,
        })
    }

    /// Construct a new matcher.
    pub(crate) fn matcher(&self) -> alloc::Result<Matcher<'_>> {
        Ok(Matcher {
            queue: [(self.root.to_path_buf(), self.components.as_ref())]
                .into_iter()
                .try_collect()?,
        })
    }
}

impl<'a> Matcher<'a> {
    /// Perform an expansion in the filesystem.
    fn expand_filesystem<M>(
        &mut self,
        path: &Path,
        rest: &'a [Component<'a>],
        mut m: M,
    ) -> Result<(), GlobError>
    where
        M: FnMut(&str) -> alloc::Result<bool>,
    {
        let io_path = if path.as_os_str().is_empty() {
            Path::new(std::path::Component::CurDir.as_os_str())
        } else {
            path
        };

        match fs::metadata(io_path) {
            Ok(m) => {
                if !m.is_dir() {
                    return Ok(());
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        }

        for e in fs::read_dir(io_path)? {
            let e = e?;
            let file_name = e.file_name();
            let c = file_name.to_string_lossy();

            if !m(c.as_ref())? {
                continue;
            }

            let mut new = path.to_path_buf();
            new.push(file_name);
            self.queue.try_push_back((new, rest))?;
        }

        Ok(())
    }

    /// Perform star star expansion.
    fn walk(&mut self, path: &Path, rest: &'a [Component<'a>]) -> Result<(), GlobError> {
        self.queue.try_push_back((path.to_path_buf(), rest))?;

        let mut queue = VecDeque::new();
        queue.try_push_back(path.to_path_buf())?;

        while let Some(path) = queue.pop_front() {
            let io_path = if path.as_os_str().is_empty() {
                Path::new(std::path::Component::CurDir.as_os_str())
            } else {
                path.as_path()
            };

            match fs::metadata(io_path) {
                Ok(m) => {
                    if !m.is_dir() {
                        return Ok(());
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    continue;
                }
                Err(e) => return Err(e.into()),
            }

            for e in fs::read_dir(io_path)? {
                let next = e?.path();
                self.queue.try_push_back((next.clone(), rest))?;
                queue.try_push_back(next)?;
            }
        }

        Ok(())
    }
}

pub(crate) struct Matcher<'a> {
    queue: VecDeque<(PathBuf, &'a [Component<'a>])>,
}

impl<'a> Iterator for Matcher<'a> {
    type Item = Result<PathBuf, GlobError>;

    fn next(&mut self) -> Option<Self::Item> {
        'outer: loop {
            let (mut path, mut components) = self.queue.pop_front()?;

            while let [first, rest @ ..] = components {
                match first {
                    Component::ParentDir => {
                        path = path.join(std::path::Component::ParentDir);
                    }
                    Component::Normal(normal) => {
                        path = path.join(normal);
                    }
                    Component::Fragment(fragment) => {
                        if let Err(e) =
                            self.expand_filesystem(&path, rest, |name| fragment.is_match(name))
                        {
                            return Some(Err(e));
                        }

                        continue 'outer;
                    }
                    Component::StarStar => {
                        if let Err(e) = self.walk(&path, rest) {
                            return Some(Err(e));
                        }

                        continue 'outer;
                    }
                }

                components = rest;
            }

            return Some(Ok(path));
        }
    }
}

#[derive(Debug, TryClone)]
enum Component<'a> {
    /// Parent directory.
    ParentDir,
    /// A normal component.
    Normal(#[try_clone(copy)] &'a str),
    /// Normal component, compiled into a fragment.
    Fragment(Fragment<'a>),
    /// `**` component, which keeps expanding.
    StarStar,
}

fn compile_pattern<P>(pattern: &P) -> alloc::Result<Vec<Component<'_>>>
where
    P: ?Sized + AsRef<RelativePath>,
{
    let pattern = pattern.as_ref();

    let mut output = Vec::new();

    for c in pattern.components() {
        output.try_push(match c {
            relative_path::Component::CurDir => continue,
            relative_path::Component::ParentDir => Component::ParentDir,
            relative_path::Component::Normal("**") => Component::StarStar,
            relative_path::Component::Normal(normal) => {
                let fragment = Fragment::parse(normal)?;

                if let Some(normal) = fragment.as_literal() {
                    Component::Normal(normal)
                } else {
                    Component::Fragment(fragment)
                }
            }
        })?;
    }

    Ok(output)
}

#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
enum Part<'a> {
    Star,
    Literal(&'a str),
}

/// A match fragment.
#[derive(Debug, TryClone)]
pub(crate) struct Fragment<'a> {
    parts: Box<[Part<'a>]>,
}

impl<'a> Fragment<'a> {
    pub(crate) fn parse(string: &'a str) -> alloc::Result<Fragment<'a>> {
        let mut literal = true;
        let mut parts = Vec::new();
        let mut start = None;

        for (n, c) in string.char_indices() {
            match c {
                '*' => {
                    if let Some(s) = start.take() {
                        parts.try_push(Part::Literal(&string[s..n]))?;
                    }

                    if mem::take(&mut literal) {
                        parts.try_push(Part::Star)?;
                    }
                }
                _ => {
                    if start.is_none() {
                        start = Some(n);
                    }

                    literal = true;
                }
            }
        }

        if let Some(s) = start {
            parts.try_push(Part::Literal(&string[s..]))?;
        }

        Ok(Fragment {
            parts: parts.try_into()?,
        })
    }

    /// Test if the given string matches the current fragment.
    pub(crate) fn is_match(&self, string: &str) -> alloc::Result<bool> {
        let mut backtrack = VecDeque::new();
        backtrack.try_push_back((self.parts.as_ref(), string))?;

        while let Some((mut parts, mut string)) = backtrack.pop_front() {
            while let Some(part) = parts.first() {
                match part {
                    Part::Star => {
                        // Peek the next literal component. If we have a
                        // trailing wildcard (which this constitutes) then it
                        // is by definition a match.
                        let Some(Part::Literal(peek)) = parts.get(1) else {
                            return Ok(true);
                        };

                        let Some(peek) = peek.chars().next() else {
                            return Ok(true);
                        };

                        while let Some(c) = string.chars().next() {
                            if c == peek {
                                backtrack.try_push_front((
                                    parts,
                                    string.get(c.len_utf8()..).unwrap_or_default(),
                                ))?;
                                break;
                            }

                            string = string.get(c.len_utf8()..).unwrap_or_default();
                        }
                    }
                    Part::Literal(literal) => {
                        // The literal component must be an exact prefix of the
                        // current string.
                        let Some(remainder) = string.strip_prefix(literal) else {
                            return Ok(false);
                        };

                        string = remainder;
                    }
                }

                parts = parts.get(1..).unwrap_or_default();
            }

            if string.is_empty() {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Treat the fragment as a single normal component.
    fn as_literal(&self) -> Option<&'a str> {
        if let [Part::Literal(one)] = self.parts.as_ref() {
            Some(one)
        } else {
            None
        }
    }
}
