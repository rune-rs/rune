//! Helper to format Rune code.

#[cfg(test)]
mod tests;

mod format;
mod output;

use core::fmt;

use crate::alloc;
use crate::alloc::prelude::*;
use crate::ast::Span;
use crate::compile::{ParseOptionError, Result, WithSpan};
use crate::grammar::{Node, Remaining, Stream, Tree};
use crate::{Diagnostics, Options, SourceId, Sources};

use self::output::Comments;
pub(crate) use self::output::Formatter;

const WS: &str = " ";
const NL: &str = "\n";
const NL_CHAR: char = '\n';
const INDENT: &str = "    ";

#[derive(Debug)]
enum FormatErrorKind {
    Build,
    ParseOptionError(ParseOptionError),
    Alloc(alloc::Error),
}

/// Error during formatting.
#[non_exhaustive]
pub struct FormatError {
    kind: FormatErrorKind,
}

impl fmt::Debug for FormatError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

impl fmt::Display for FormatError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.kind {
            FormatErrorKind::Build => write!(f, "Failed to format source"),
            FormatErrorKind::ParseOptionError(error) => error.fmt(f),
            FormatErrorKind::Alloc(error) => error.fmt(f),
        }
    }
}

impl From<ParseOptionError> for FormatError {
    fn from(value: ParseOptionError) -> Self {
        Self {
            kind: FormatErrorKind::ParseOptionError(value),
        }
    }
}

impl From<alloc::Error> for FormatError {
    fn from(value: alloc::Error) -> Self {
        Self {
            kind: FormatErrorKind::Alloc(value),
        }
    }
}

impl core::error::Error for FormatError {}

/// Format the given source.
pub fn prepare(sources: &Sources) -> Prepare<'_> {
    Prepare {
        sources,
        options: None,
        diagnostics: None,
    }
}

/// A prepared formatting operation.
///
/// See [prepare].
pub struct Prepare<'a> {
    sources: &'a Sources,
    options: Option<&'a Options>,
    diagnostics: Option<&'a mut Diagnostics>,
}

impl<'a> Prepare<'a> {
    /// Associate diagnostics with the build.
    pub fn with_diagnostics(mut self, diagnostics: &'a mut Diagnostics) -> Self {
        self.diagnostics = Some(diagnostics);
        self
    }

    /// Associate options with the build.
    pub fn with_options(mut self, options: &'a Options) -> Self {
        self.options = Some(options);
        self
    }

    /// Format the given sources.
    pub fn format(self) -> Result<Vec<(SourceId, String)>, FormatError> {
        let mut local;

        let diagnostics = match self.diagnostics {
            Some(diagnostics) => diagnostics,
            None => {
                local = Diagnostics::new();
                &mut local
            }
        };

        let default_options;

        let options = match self.options {
            Some(options) => options,
            None => {
                default_options = Options::from_default_env()?;
                &default_options
            }
        };

        let mut files = Vec::new();

        for id in self.sources.source_ids() {
            let Some(source) = self.sources.get(id) else {
                continue;
            };

            match layout_source_with(source.as_str(), id, options, diagnostics) {
                Ok(output) => {
                    files.try_push((id, output))?;
                }
                Err(error) => {
                    diagnostics.error(id, error)?;
                }
            }
        }

        if diagnostics.has_error() {
            return Err(FormatError {
                kind: FormatErrorKind::Build,
            });
        }

        Ok(files)
    }
}

/// Format the given source with the specified options.
pub(crate) fn layout_source_with(
    source: &str,
    source_id: SourceId,
    options: &Options,
    diagnostics: &mut Diagnostics,
) -> Result<String> {
    let tree = crate::grammar::text(source_id, source)
        .without_processing()
        .include_whitespace()
        .root()?;

    #[cfg(feature = "std")]
    if options.print_tree {
        tree.print_with_source(
            &Span::empty(),
            format_args!("Formatting source #{source_id}"),
            source,
        )?;
    }

    let mut o = String::new();

    {
        let mut o = Formatter::new(
            Span::new(0, 0),
            source,
            source_id,
            &mut o,
            &options.fmt,
            diagnostics,
        );

        o.flush_prefix_comments(&tree)?;
        format::root(&mut o, &tree)?;
        o.comments(Comments::Line)?;
    }

    if options.fmt.force_newline && !o.ends_with(NL) {
        o.try_push_str(NL)
            .with_span(Span::new(source.len(), source.len()))?;
    }

    Ok(o)
}
