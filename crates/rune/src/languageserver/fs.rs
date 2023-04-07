use std::io;
use std::path::Path;

/// Test if the given path is a file.
pub(super) async fn is_file<P>(path: P) -> io::Result<bool>
where
    P: AsRef<Path>,
{
    match tokio::fs::metadata(path).await {
        Ok(m) => Ok(m.is_file()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}
