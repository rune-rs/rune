// Implementation copied and adjusted from https://github.com/servo/rust-url

use std::fmt::Write;
use std::path::Path;

use anyhow::anyhow;

use crate::no_std::prelude::*;
use crate::support::Result;

use percent_encoding::{percent_encode, AsciiSet, CONTROLS};
use url::Url;

/// https://url.spec.whatwg.org/#fragment-percent-encode-set
const FRAGMENT: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');
/// https://url.spec.whatwg.org/#path-percent-encode-set
const PATH: &AsciiSet = &FRAGMENT.add(b'#').add(b'?').add(b'{').add(b'}');
const PATH_SEGMENT: &AsciiSet = &PATH.add(b'/').add(b'%');

/// Convert a file path into a URL.
pub(super) fn from_file_path<P>(path: P) -> Result<Url>
where
    P: AsRef<Path>,
{
    let mut buf = "file://".to_owned();

    let Ok(()) = path_to_file_url_segments(path.as_ref(), &mut buf) else {
        return Err(anyhow!("failed to construct file segments"));
    };

    Ok(Url::parse(&buf)?)
}

#[cfg(any(unix, target_os = "redox", target_os = "wasi"))]
fn path_to_file_url_segments(path: &Path, buf: &mut String) -> Result<(), ()> {
    #[cfg(any(unix, target_os = "redox"))]
    use std::os::unix::prelude::OsStrExt;
    #[cfg(target_os = "wasi")]
    use std::os::wasi::prelude::OsStrExt;

    if !path.is_absolute() {
        return Err(());
    }

    let mut empty = true;

    // skip the root component
    for component in path.components().skip(1) {
        empty = false;
        buf.push('/');
        buf.extend(percent_encode(
            component.as_os_str().as_bytes(),
            PATH_SEGMENT,
        ));
    }

    if empty {
        // An URL’s path must not be empty.
        buf.push('/');
    }

    Ok(())
}

#[cfg(windows)]
fn path_to_file_url_segments(path: &Path, buf: &mut String) -> Result<(), ()> {
    path_to_file_url_segments_windows(path, buf)
}

// Build this unconditionally to alleviate https://github.com/servo/rust-url/issues/102
#[cfg_attr(not(windows), allow(dead_code))]
fn path_to_file_url_segments_windows(path: &Path, buf: &mut String) -> Result<(), ()> {
    use std::path::{Component, Prefix};

    if !path.is_absolute() {
        return Err(());
    }

    let mut components = path.components();

    match components.next() {
        Some(Component::Prefix(ref p)) => match p.kind() {
            Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => {
                buf.push('/');
                buf.push((letter as char).to_ascii_lowercase());
                buf.push_str("%3A");
            }
            Prefix::UNC(server, share) | Prefix::VerbatimUNC(server, share) => {
                let host = url::Host::parse(server.to_str().ok_or(())?).map_err(|_| ())?;
                write!(buf, "{}", host).map_err(|_| ())?;
                buf.push('/');
                let share = share.to_str().ok_or(())?;
                buf.extend(percent_encode(share.as_bytes(), PATH_SEGMENT));
            }
            _ => return Err(()),
        },
        _ => return Err(()),
    }

    let mut only_prefix = true;

    for component in components {
        if component == Component::RootDir {
            continue;
        }

        only_prefix = false;
        let component = component.as_os_str().to_str().ok_or(())?;

        buf.push('/');
        buf.extend(percent_encode(component.as_bytes(), PATH_SEGMENT));
    }

    if only_prefix {
        buf.push('/');
    }

    Ok(())
}
