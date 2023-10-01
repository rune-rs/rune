// Implementation copied and adjusted from https://github.com/servo/rust-url

use std::path::Path;

use anyhow::anyhow;

use crate::alloc::fmt::TryWrite;
use crate::alloc::prelude::*;
use crate::alloc::String;
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
    let mut buf = "file://".try_to_owned()?;
    path_to_file_url_segments(path.as_ref(), &mut buf)?;
    Ok(Url::parse(&buf)?)
}

#[cfg(any(unix, target_os = "redox", target_os = "wasi"))]
fn path_to_file_url_segments(path: &Path, buf: &mut String) -> Result<()> {
    #[cfg(any(unix, target_os = "redox"))]
    use std::os::unix::prelude::OsStrExt;
    #[cfg(target_os = "wasi")]
    use std::os::wasi::prelude::OsStrExt;

    if !path.is_absolute() {
        return Err(anyhow!("Path must be absolute"));
    }

    let mut empty = true;

    // skip the root component
    for component in path.components().skip(1) {
        empty = false;
        buf.try_push('/')?;
        buf.try_extend(percent_encode(
            component.as_os_str().as_bytes(),
            PATH_SEGMENT,
        ))?;
    }

    if empty {
        // An URL’s path must not be empty.
        buf.try_push('/')?;
    }

    Ok(())
}

#[cfg(windows)]
fn path_to_file_url_segments(path: &Path, buf: &mut String) -> Result<()> {
    path_to_file_url_segments_windows(path, buf)
}

// Build this unconditionally to alleviate https://github.com/servo/rust-url/issues/102
#[cfg_attr(not(windows), allow(dead_code))]
fn path_to_file_url_segments_windows(path: &Path, buf: &mut String) -> Result<()> {
    use std::path::{Component, Prefix};

    if !path.is_absolute() {
        return Err(anyhow!("Path must be absolute"));
    }

    let mut components = path.components();

    match components.next() {
        Some(Component::Prefix(ref p)) => match p.kind() {
            Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => {
                buf.try_push('/')?;
                buf.try_push((letter as char).to_ascii_lowercase())?;
                buf.try_push_str("%3A")?;
            }
            Prefix::UNC(server, share) | Prefix::VerbatimUNC(server, share) => {
                let Some(server) = server.to_str() else {
                    return Err(anyhow!("UNC server is not valid UTF-8"));
                };

                let host = url::Host::parse(server)?;
                write!(buf, "{}", host)?;
                buf.try_push('/')?;

                let Some(share) = share.to_str() else {
                    return Err(anyhow!("UNC share is not valid UTF-8"));
                };

                buf.try_extend(percent_encode(share.as_bytes(), PATH_SEGMENT))?;
            }
            _ => return Err(anyhow!("Illegal path component")),
        },
        _ => return Err(anyhow!("Illegal path component")),
    }

    let mut only_prefix = true;

    for component in components {
        if component == Component::RootDir {
            continue;
        }

        only_prefix = false;

        let Some(component) = component.as_os_str().to_str() else {
            return Err(anyhow!("Path component is not valid UTF-8"));
        };

        buf.try_push('/')?;
        buf.try_extend(percent_encode(component.as_bytes(), PATH_SEGMENT))?;
    }

    if only_prefix {
        buf.try_push('/')?;
    }

    Ok(())
}
