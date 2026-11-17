//! A preview server for the built site, which renders it again when anything
//! under `site/` has changed since the last build.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::SystemTime;

use anyhow::Result;

pub fn run(site: &Path, out: &Path, bind: &str, port: u16) -> Result<()> {
    let listener = TcpListener::bind((bind, port))?;
    println!("Serving {} on http://{bind}:{port}/", out.display());

    let built = Arc::new(Mutex::new(modified(site)));

    for stream in listener.incoming() {
        let Ok(stream) = stream else {
            continue;
        };

        let site = site.to_owned();
        let out = out.to_owned();
        let built = built.clone();

        thread::spawn(move || {
            if let Err(error) = handle(stream, &site, &out, &built) {
                eprintln!("error: {error}");
            }
        });
    }

    Ok(())
}

fn handle(mut stream: TcpStream, site: &Path, out: &Path, built: &Mutex<SystemTime>) -> Result<()> {
    let mut line = String::new();
    let mut reader = BufReader::new(stream.try_clone()?);
    reader.read_line(&mut line)?;

    // The rest of the request is not needed, but is read so that the client
    // does not see the connection reset under it.
    loop {
        let mut header = String::new();

        if reader.read_line(&mut header)? == 0 || header == "\r\n" || header == "\n" {
            break;
        }
    }

    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let target = parts.next().unwrap_or("/");
    let path = target.split(['?', '#']).next().unwrap_or("/");

    if method != "GET" && method != "HEAD" {
        return respond(&mut stream, "405 Method Not Allowed", "text/plain", b"");
    }

    let extension = Path::new(path).extension().and_then(|e| e.to_str());

    if matches!(extension, None | Some("html" | "json")) {
        let mut built = built.lock().unwrap_or_else(|e| e.into_inner());
        let now = modified(site);

        if now > *built {
            match crate::build(site, out) {
                Ok(()) => println!("Rebuilt after a change"),
                Err(error) => eprintln!("error: rebuilding: {error:?}"),
            }

            *built = now;
        }
    }

    let Some(file) = resolve(out, path) else {
        let body = fs::read(out.join("404.html")).unwrap_or_default();
        return respond(
            &mut stream,
            "404 Not Found",
            "text/html; charset=utf-8",
            &body,
        );
    };

    let body = fs::read(&file)?;
    let body = if method == "HEAD" { &[][..] } else { &body[..] };
    respond(&mut stream, "200 OK", content_type(&file), body)
}

fn resolve(out: &Path, path: &str) -> Option<PathBuf> {
    let mut file = out.to_owned();

    for component in Path::new(path.trim_start_matches('/')).components() {
        match component {
            Component::Normal(part) => file.push(part),
            Component::CurDir => {}
            _ => return None,
        }
    }

    if file.is_dir() {
        file.push("index.html");
    }

    file.is_file().then_some(file)
}

fn respond(stream: &mut TcpStream, status: &str, content_type: &str, body: &[u8]) -> Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    Ok(())
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("wasm") => "application/wasm",
        Some("xml") => "application/xml",
        Some("png") => "image/png",
        Some("svg") => "image/svg+xml",
        Some("gif") => "image/gif",
        Some("jpg" | "jpeg") => "image/jpeg",
        _ => "application/octet-stream",
    }
}

/// The newest modification under `dir`.
fn modified(dir: &Path) -> SystemTime {
    let mut newest = SystemTime::UNIX_EPOCH;
    let mut queue = vec![dir.to_owned()];

    while let Some(dir) = queue.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                queue.push(path);
            } else if let Ok(time) = entry.metadata().and_then(|m| m.modified()) {
                newest = newest.max(time);
            }
        }
    }

    newest
}
