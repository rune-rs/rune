use std::io::Write;

use anyhow::{Context, Result};
use clap::Parser;

use crate::cli::{ExitCode, Io, EntryPoint};
use crate::termcolor::WriteColor;
use crate::{Source};

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

pub(super) fn run<'m, I>(io: &mut Io<'_>, entrys: I, flags: &Flags) -> Result<ExitCode> where I: IntoIterator<Item = EntryPoint<'m>> {
    let mut red = crate::termcolor::ColorSpec::new();
    red.set_fg(Some(crate::termcolor::Color::Red));

    let mut green = crate::termcolor::ColorSpec::new();
    green.set_fg(Some(crate::termcolor::Color::Green));

    let mut yellow = crate::termcolor::ColorSpec::new();
    yellow.set_fg(Some(crate::termcolor::Color::Yellow));

    let mut succeeded = 0;
    let mut failed = 0;
    let mut unchanged = 0;

    for e in entrys {
        let source =
            Source::from_path(e.path()).with_context(|| format!("reading file: {}", e.path().display()))?;

        match crate::fmt::layout_source(&source) {
            Ok(val) => {
                if val == source.as_str() {
                    if !flags.check {
                        io.stdout.set_color(&yellow)?;
                        write!(io.stdout, "== ")?;
                        io.stdout.reset()?;
                        writeln!(io.stdout, "{}", e.path().display())?;
                    }

                    unchanged += 1;
                } else {
                    succeeded += 1;
                    io.stdout.set_color(&green)?;
                    write!(io.stdout, "++ ")?;
                    io.stdout.reset()?;
                    writeln!(io.stdout, "{}", e.path().display())?;
                    if !flags.check {
                        std::fs::write(e.path(), &val)?;
                    }
                }
            }
            Err(err) => {
                failed += 1;
                io.stdout.set_color(&red)?;
                write!(io.stdout, "!! ")?;
                io.stdout.reset()?;
                writeln!(io.stdout, "{}: {}", e.path().display(), err)?;
            }
        }
    }

    io.stdout.set_color(&yellow)?;
    write!(io.stdout, "{}", unchanged)?;
    io.stdout.reset()?;
    writeln!(io.stdout, " unchanged")?;
    io.stdout.set_color(&green)?;
    write!(io.stdout, "{}", succeeded)?;
    io.stdout.reset()?;
    writeln!(io.stdout, " succeeded")?;
    io.stdout.set_color(&red)?;
    write!(io.stdout, "{}", failed)?;
    io.stdout.reset()?;
    writeln!(io.stdout, " failed")?;

    if flags.check && succeeded > 0 {
        io.stdout.set_color(&red)?;
        write!(
            io.stdout,
            "Exiting with failure due to `--check` flag and unformatted files."
        )?;
        io.stdout.reset()?;
        return Ok(ExitCode::Failure);
    }

    if failed > 0 {
        return Ok(ExitCode::Failure);
    }

    Ok(ExitCode::Success)
}
