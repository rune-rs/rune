use std::mem;
use std::ptr;

use rune::termcolor as t;

/// Internal handle for output stream.
pub(crate) type InternalStandardStream = Option<t::StandardStream>;

/// A standard stream.
#[repr(C)]
pub struct StandardStream {
    #[cfg(windows)]
    repr: [u8; 88],
    #[cfg(not(windows))]
    repr: [u8; 56],
}

test_size!(StandardStream, InternalStandardStream);

/// The color choice.
#[repr(usize)]
pub enum ColorChoice {
    /// Try very hard to emit colors. This includes emitting ANSI colors on
    /// Windows if the console API is unavailable.
    ALWAYS = 1,
    /// AlwaysAnsi is like Always, except it never tries to use anything other
    /// than emitting ANSI color codes.
    ALWAYS_ANSI = 2,
    /// Try to use colors, but don't force the issue. If the console isn't
    /// available on Windows, or if TERM=dumb, or if `NO_COLOR` is defined, for
    /// example, then don't use colors.
    AUTO = 3,
    /// Never emit colors.
    NEVER = 4,
}

fn color_choice_convert(color_choice: ColorChoice) -> t::ColorChoice {
    match color_choice {
        ColorChoice::ALWAYS => t::ColorChoice::Always,
        ColorChoice::ALWAYS_ANSI => t::ColorChoice::AlwaysAnsi,
        ColorChoice::AUTO => t::ColorChoice::Auto,
        _ => t::ColorChoice::Never,
    }
}

/// Construct a standard stream for stdout.
#[no_mangle]
pub extern "C" fn rune_standard_stream_stdout(color_choice: ColorChoice) -> StandardStream {
    let color_choice = color_choice_convert(color_choice);

    // Safety: this allocation is safe.
    unsafe { mem::transmute(Some(t::StandardStream::stdout(color_choice))) }
}

/// Construct a standard stream for stderr.
#[no_mangle]
pub extern "C" fn rune_standard_stream_stderr(color_choice: ColorChoice) -> StandardStream {
    let color_choice = color_choice_convert(color_choice);

    // Safety: this allocation is safe.
    unsafe { mem::transmute(Some(t::StandardStream::stderr(color_choice))) }
}

/// Free a standard stream.
///
/// # Safety
///
/// This must be called with a `standard_stream` that has been allocated with
/// functions such as [rune_standard_stream_stdout] or
/// [rune_standard_stream_stderr].
#[no_mangle]
pub unsafe extern "C" fn rune_standard_stream_free(standard_stream: *mut StandardStream) {
    let _ = ptr::replace(standard_stream as *mut InternalStandardStream, None);
}
