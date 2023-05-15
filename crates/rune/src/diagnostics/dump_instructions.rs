use crate::runtime::Unit;
use crate::{Source, Sources, Span};
use codespan_reporting::term::termcolor;
use codespan_reporting::term::termcolor::WriteColor;
use std::io;

/// Trait to dump the instructions of a unit to the given writer.
///
/// This is implemented for [Unit].
pub trait DumpInstructions {
    /// Dump the instructions of the current unit to the given writer.
    fn dump_instructions<O>(
        &self,
        out: &mut O,
        sources: &Sources,
        with_source: bool,
    ) -> io::Result<()>
    where
        O: WriteColor;
}

impl DumpInstructions for Unit {
    fn dump_instructions<O>(
        &self,
        out: &mut O,
        sources: &Sources,
        with_source: bool,
    ) -> io::Result<()>
    where
        O: WriteColor,
    {
        let mut first_function = true;

        for (n, inst) in self.iter_instructions() {
            let debug = self.debug_info().and_then(|d| d.instruction_at(n));

            if let Some((hash, signature)) = self.debug_info().and_then(|d| d.function_at(n)) {
                if !std::mem::take(&mut first_function) {
                    writeln!(out)?;
                }

                writeln!(out, "fn {} ({}):", signature, hash)?;
            }

            if with_source {
                if let Some((source, span)) =
                    debug.and_then(|d| sources.get(d.source_id).map(|s| (s, d.span)))
                {
                    source.emit_source_line(out, span)?;
                }
            }

            if let Some(label) = debug.and_then(|d| d.label.as_ref()) {
                writeln!(out, "{}:", label)?;
            }

            write!(out, "  {:04} = {}", n, inst)?;

            if let Some(comment) = debug.and_then(|d| d.comment.as_ref()) {
                write!(out, " // {}", comment)?;
            }

            writeln!(out)?;
        }

        Ok(())
    }
}

/// Helper trait to emit source code locations.
///
/// These are implemented for [Source], so that you can print diagnostics about
/// a source conveniently.
pub trait EmitSource {
    /// Emit the source location as a single line of the given writer.
    fn emit_source_line<O>(&self, out: &mut O, span: Span) -> io::Result<()>
    where
        O: WriteColor;
}

impl EmitSource for Source {
    fn emit_source_line<O>(&self, out: &mut O, span: Span) -> io::Result<()>
    where
        O: WriteColor,
    {
        let mut highlight = termcolor::ColorSpec::new();
        highlight.set_fg(Some(termcolor::Color::Blue));

        let diagnostics = line_for(self, span);

        if let Some((count, line, span)) = diagnostics {
            let line = line.trim_end();
            let end = usize::min(span.end.into_usize(), line.len());

            let before = &line[0..span.start.into_usize()];
            let inner = &line[span.start.into_usize()..end];
            let after = &line[end..];

            write!(out, "  {}:{: <3} - {}", self.name(), count + 1, before,)?;

            out.set_color(&highlight)?;
            write!(out, "{}", inner)?;
            out.reset()?;
            write!(out, "{}", after)?;

            if span.end != end {
                write!(out, " .. trimmed")?;
            }

            writeln!(out)?;
        }

        Ok(())
    }
}

/// Get the line number and source line for the given source and span.
pub fn line_for(source: &Source, span: Span) -> Option<(usize, &str, Span)> {
    let line_starts = source.line_starts();

    let line = match line_starts.binary_search(&span.start.into_usize()) {
        Ok(n) => n,
        Err(n) => n.saturating_sub(1),
    };

    let start = *line_starts.get(line)?;
    let end = line.checked_add(1)?;

    let s = if let Some(end) = line_starts.get(end) {
        source.get(start..*end)?
    } else {
        source.get(start..)?
    };

    Some((
        line,
        s,
        Span::new(
            span.start.into_usize() - start,
            span.end.into_usize() - start,
        ),
    ))
}
