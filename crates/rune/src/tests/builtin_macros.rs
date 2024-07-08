#![cfg(feature = "capture-io")]

prelude!();

use crate::termcolor::{ColorChoice, StandardStream};

macro_rules! capture {
    ($($tt:tt)*) => {{
        let capture = crate::modules::capture_io::CaptureIo::new();
        let module = crate::modules::capture_io::module(&capture).context("building capture module")?;

        let mut context = Context::with_config(false).context("building context")?;
        context.install(module).context("installing module")?;

        let source = Source::memory(concat!("pub fn main() { ", stringify!($($tt)*), " }")).context("building source")?;

        let mut sources = Sources::new();
        sources.insert(source).context("inserting source")?;

        let mut diagnostics = Diagnostics::new();

        let unit = prepare(&mut sources).with_context(&context).with_diagnostics(&mut diagnostics).build();

        if !diagnostics.is_empty() {
            let mut writer = StandardStream::stderr(ColorChoice::Always);
            diagnostics.emit(&mut writer, &sources)?;
        }

        let unit = Arc::new(unit.context("building unit")?);

        let context = context.runtime().context("constructing runtime context")?;
        let context = Arc::new(context);

        let mut vm = Vm::new(context, unit);
        vm.call(["main"], ()).context("calling main")?;
        capture.drain_utf8().context("draining utf-8 capture")?
    }};
}

macro_rules! test_case {
    ($expected:expr, {$($prefix:tt)*}, $($format:tt)*) => {{
        let string = capture!($($prefix)* println!($($format)*));
        assert_eq!(string, concat!($expected, "\n"), "Expecting println!");

        let string = capture!($($prefix)* print!($($format)*));
        assert_eq!(string, $expected, "Expecting print!");

        let string: String = rune!(pub fn main() { $($prefix)* format!($($format)*) });
        assert_eq!(string, $expected, "Expecting format!");
    }}
}

#[test]
fn format_macros() -> Result<()> {
    test_case!("Hello World!", {}, "Hello World!");
    test_case!("Hello World!", {}, "Hello {}!", "World");
    test_case!(
        "Hello World!",
        {
            let pos = "Hello";
        },
        "{pos} {}!",
        "World"
    );
    test_case!(
        "Hello World!",
        {
            let pos = "Not Hello";
        },
        "{pos} {}!",
        "World",
        pos = "Hello"
    );
    Ok(())
}
