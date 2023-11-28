use core::fmt;

use std::io::Write;

use clap::Parser;
use similar::{ChangeTag, TextDiff};

use crate::alloc::prelude::*;
use crate::alloc::BTreeSet;
use crate::cli::{AssetKind, CommandBase, Config, Entry, EntryPoint, ExitCode, Io, SharedFlags};
use crate::support::{Context, Result};
use crate::termcolor::{Color, ColorSpec, WriteColor};
use crate::{Diagnostics, Options, Source, Sources};

#[derive(Parser, Debug)]
pub(super) struct Flags {
    /// Exit with a non-zero exit-code even for warnings
    #[arg(long)]
    warnings_are_errors: bool,
    /// Perform format checking. If there's any files which needs to be changed
    /// returns a non-successful exitcode.
    #[arg(long)]
    check: bool,
}

impl CommandBase for Flags {
    #[inline]
    fn is_workspace(&self, _: AssetKind) -> bool {
        true
    }

    #[inline]
    fn describe(&self) -> &str {
        "Formatting"
    }
}

pub(super) fn run<'m, I>(
    io: &mut Io<'_>,
    entry: &mut Entry<'_>,
    c: &Config,
    entrys: I,
    flags: &Flags,
    shared: &SharedFlags,
    options: &Options,
) -> Result<ExitCode>
where
    I: IntoIterator<Item = EntryPoint<'m>>,
{
    let col = Colors::new();

    let mut changed = 0;
    let mut failed = 0;
    let mut unchanged = 0;
    let mut failed_builds = 0;

    let context = shared.context(entry, c, None)?;

    let mut paths = BTreeSet::new();

    for e in entrys {
        let mut diagnostics = if shared.warnings || flags.warnings_are_errors {
            Diagnostics::new()
        } else {
            Diagnostics::without_warnings()
        };

        let mut sources = Sources::new();

        sources.insert(match Source::from_path(e.path()) {
            Ok(source) => source,
            Err(error) => return Err(error).context(e.path().display().try_to_string()?),
        })?;

        let _ = crate::prepare(&mut sources)
            .with_context(&context)
            .with_diagnostics(&mut diagnostics)
            .with_options(options)
            .build();

        diagnostics.emit(&mut io.stdout.lock(), &sources)?;

        if diagnostics.has_error() || flags.warnings_are_errors && diagnostics.has_warning() {
            failed_builds += 1;
        }

        for source in sources.iter() {
            if let Some(path) = source.path() {
                paths.try_insert(path.try_to_owned()?)?;
            }
        }
    }

    for path in paths {
        let source = match Source::from_path(&path) {
            Ok(source) => source,
            Err(error) => return Err(error).context(path.display().try_to_string()?),
        };

        let val = match crate::fmt::layout_source(source.as_str()) {
            Ok(val) => val,
            Err(err) => {
                failed += 1;

                if shared.verbose {
                    io.stdout.set_color(&col.red)?;
                    write!(io.stdout, "!! ")?;
                    io.stdout.reset()?;
                    writeln!(io.stdout, "{}: {}", path.display(), err)?;
                }

                continue;
            }
        };

        let same = source.as_str().as_bytes() == val;

        if same {
            unchanged += 1;

            if shared.verbose {
                io.stdout.set_color(&col.green)?;
                write!(io.stdout, "== ")?;
                io.stdout.reset()?;
                writeln!(io.stdout, "{}", path.display())?;
            }

            continue;
        }

        changed += 1;

        if shared.verbose || flags.check {
            io.stdout.set_color(&col.yellow)?;
            write!(io.stdout, "++ ")?;
            io.stdout.reset()?;
            writeln!(io.stdout, "{}", path.display())?;
            diff(io, &source, &val, &col)?;
        }

        if !flags.check {
            std::fs::write(path, &val)?;
        }
    }

    if shared.verbose && unchanged > 0 {
        io.stdout.set_color(&col.green)?;
        write!(io.stdout, "{}", unchanged)?;
        io.stdout.reset()?;
        writeln!(io.stdout, " unchanged")?;
    }

    if shared.verbose && changed > 0 {
        io.stdout.set_color(&col.yellow)?;
        write!(io.stdout, "{}", changed)?;
        io.stdout.reset()?;
        writeln!(io.stdout, " changed")?;
    }

    if shared.verbose && failed > 0 {
        io.stdout.set_color(&col.red)?;
        write!(io.stdout, "{}", failed)?;
        io.stdout.reset()?;
        writeln!(io.stdout, " failed")?;
    }

    if shared.verbose && failed_builds > 0 {
        io.stdout.set_color(&col.red)?;
        write!(io.stdout, "{}", failed_builds)?;
        io.stdout.reset()?;
        writeln!(io.stdout, " failed builds")?;
    }

    if flags.check && changed > 0 {
        io.stdout.set_color(&col.red)?;
        writeln!(
            io.stdout,
            "Failure due to `--check` flag and unformatted files."
        )?;
        io.stdout.reset()?;
        return Ok(ExitCode::Failure);
    }

    if failed > 0 || failed_builds > 0 {
        return Ok(ExitCode::Failure);
    }

    Ok(ExitCode::Success)
}

fn diff(io: &mut Io, source: &Source, val: &[u8], col: &Colors) -> Result<(), anyhow::Error> {
    let diff = TextDiff::from_lines(source.as_str().as_bytes(), val);

    for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
        if idx > 0 {
            println!("{:-^1$}", "-", 80);
        }

        for op in group {
            for change in diff.iter_inline_changes(op) {
                let (sign, color) = match change.tag() {
                    ChangeTag::Delete => ("-", &col.red),
                    ChangeTag::Insert => ("+", &col.green),
                    ChangeTag::Equal => (" ", &col.dim),
                };

                io.stdout.set_color(color)?;

                write!(io.stdout, "{}", Line(change.old_index()))?;
                write!(io.stdout, "{sign}")?;

                for (_, value) in change.iter_strings_lossy() {
                    write!(io.stdout, "{value}")?;
                }

                io.stdout.reset()?;

                if change.missing_newline() {
                    writeln!(io.stdout)?;
                }
            }
        }
    }

    Ok(())
}

struct Line(Option<usize>);

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            None => write!(f, "    "),
            Some(idx) => write!(f, "{:<4}", idx + 1),
        }
    }
}

struct Colors {
    red: ColorSpec,
    green: ColorSpec,
    yellow: ColorSpec,
    dim: ColorSpec,
}

impl Colors {
    fn new() -> Self {
        let mut this = Self {
            red: ColorSpec::new(),
            green: ColorSpec::new(),
            yellow: ColorSpec::new(),
            dim: ColorSpec::new(),
        };

        this.red.set_fg(Some(Color::Red));
        this.green.set_fg(Some(Color::Green));
        this.yellow.set_fg(Some(Color::Yellow));

        this
    }
}
