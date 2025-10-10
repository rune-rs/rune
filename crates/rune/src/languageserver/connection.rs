use core::fmt;

#[cfg(feature = "std")]
use tokio::io::{Stdin, Stdout};

use anyhow::{anyhow, bail, Result};
use tokio::io::{
    self, AsyncBufRead, AsyncBufReadExt as _, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _,
    BufReader,
};

use crate::alloc::prelude::*;
use crate::languageserver::envelope;

/// An input frame.
#[derive(Debug)]
pub(super) struct Frame<'a> {
    pub(super) content: &'a [u8],
}

/// Input connection.
pub struct Input<I> {
    buf: rust_alloc::vec::Vec<u8>,
    reader: I,
}

impl<I> Input<I> {
    /// Create a new input connection.
    pub fn new(reader: I) -> Self {
        Self {
            buf: rust_alloc::vec::Vec::new(),
            reader,
        }
    }
}

#[cfg(feature = "std")]
impl Input<BufReader<Stdin>> {
    /// Create a new input connection from stdin.
    pub fn from_stdin() -> Self {
        let stdin = io::stdin();
        let reader = BufReader::new(stdin);
        Self::new(reader)
    }
}

impl<I> Input<I>
where
    I: Unpin + AsyncBufRead,
{
    /// Get the next input frame.
    pub(super) async fn next(&mut self) -> Result<Option<Frame<'_>>> {
        let headers = match Headers::read(&mut self.buf, &mut self.reader).await? {
            Some(headers) => headers,
            None => return Ok(None),
        };

        tracing::trace!("headers: {:?}", headers);

        let length = match headers.content_length {
            Some(length) => length as usize,
            None => bail!("missing content-length"),
        };

        self.buf.resize(length, 0u8);
        self.reader.read_exact(&mut self.buf[..]).await?;
        Ok(Some(Frame { content: &self.buf }))
    }
}

/// Buffer for outbound data.
pub(crate) struct Outbound {
    scratch: rust_alloc::vec::Vec<u8>,
    buf: rust_alloc::vec::Vec<u8>,
    write: usize,
}

impl Outbound {
    pub(crate) fn new() -> Self {
        Self {
            scratch: rust_alloc::vec::Vec::new(),
            buf: rust_alloc::vec::Vec::new(),
            write: 0,
        }
    }

    /// Check if the buffer is empty.
    pub(crate) fn is_empty(&self) -> bool {
        self.write >= self.buf.len()
    }

    /// Get slice of readable data.
    pub(crate) fn readable(&self) -> &[u8] {
        self.buf.get(self.write..).unwrap_or_default()
    }

    /// Advance the write position by the given amount.
    pub(crate) fn advance(&mut self, n: usize) {
        self.write += n;

        if self.write >= self.buf.len() {
            debug_assert_eq!(self.write, self.buf.len());
            self.buf.clear();
            self.write = 0;
        }
    }

    /// Write the given response.
    pub(super) fn response<R>(&mut self, id: Option<envelope::RequestId>, result: R) -> Result<()>
    where
        R: serde::Serialize,
    {
        let response = envelope::ResponseMessage {
            jsonrpc: envelope::V2,
            id,
            result: Some(result),
            error: None::<envelope::ResponseError<()>>,
        };

        serde_json::to_writer(&mut self.scratch, &response)?;
        self.write_buf()?;
        Ok(())
    }

    /// Write that the given method is not supported.
    pub(super) fn method_not_found(&mut self, id: Option<envelope::RequestId>) -> Result<()> {
        self.error(
            id,
            envelope::Code::MethodNotFound,
            "Method not found",
            None::<()>,
        )?;
        Ok(())
    }

    /// Write the given error as response.
    pub(super) fn error<D>(
        &mut self,
        id: Option<envelope::RequestId>,
        code: envelope::Code,
        message: impl AsRef<str>,
        data: Option<D>,
    ) -> Result<()>
    where
        D: serde::Serialize,
    {
        let message = message.as_ref();

        tracing::error!(?code, "{message}");

        let response = envelope::ResponseMessage {
            jsonrpc: envelope::V2,
            id,
            result: None::<()>,
            error: Some(envelope::ResponseError {
                code,
                message,
                data,
            }),
        };

        serde_json::to_writer(&mut self.scratch, &response)?;
        self.write_buf()?;
        Ok(())
    }

    /// Write the given notification
    pub(super) fn notification<N>(&mut self, notification: N::Params) -> Result<()>
    where
        N: lsp::notification::Notification,
    {
        let notification = envelope::NotificationMessage {
            jsonrpc: envelope::V2,
            method: N::METHOD,
            params: notification,
        };

        serde_json::to_writer(&mut self.scratch, &notification)?;
        self.write_buf()?;
        Ok(())
    }

    /// Write a log message.
    pub(super) fn log<M>(&mut self, typ: lsp::MessageType, message: M) -> Result<()>
    where
        M: fmt::Display,
    {
        match typ {
            lsp::MessageType::ERROR => tracing::error!("LOG: {message}"),
            lsp::MessageType::WARNING => tracing::warn!("LOG: {message}"),
            lsp::MessageType::INFO => tracing::info!("LOG: {message}"),
            lsp::MessageType::LOG => tracing::debug!("LOG: {message}"),
            _ => tracing::debug!("LOG: {message}"),
        }

        self.notification::<lsp::notification::LogMessage>(lsp::LogMessageParams {
            typ,
            message: message.try_to_string()?.into_std(),
        })?;

        Ok(())
    }

    /// Write the given response body based on the scratch buffer.
    fn write_buf(&mut self) -> Result<()> {
        use std::io::Write as _;

        write!(self.buf, "Content-Length: {}\r\n", self.scratch.len())?;
        write!(self.buf, "\r\n")?;
        self.buf.extend_from_slice(&self.scratch);
        self.scratch.clear();
        Ok(())
    }
}

/// Output connection.
pub struct Output<W> {
    writer: W,
}

impl<W> Output<W> {
    /// Create a new output connection.
    pub fn new(writer: W) -> Self {
        Self { writer }
    }
}

#[cfg(feature = "std")]
impl Output<Stdout> {
    /// Create a new output connection from stdout.
    pub fn from_stdout() -> Self {
        let stdout = io::stdout();
        Self::new(stdout)
    }
}

impl<W> Output<W>
where
    W: Unpin + AsyncWrite,
{
    #[inline]
    pub(crate) async fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.writer.write(buf).await
    }

    #[inline]
    pub(crate) async fn flush(&mut self) -> io::Result<()> {
        self.writer.flush().await
    }
}

#[derive(Debug)]
pub(super) enum ContentType {
    JsonRPC,
}

#[derive(Default, Debug)]
pub(super) struct Headers {
    pub(super) content_length: Option<u32>,
    pub(super) content_type: Option<ContentType>,
}

impl Headers {
    /// Read headers from the given line stream.
    pub(super) async fn read<S>(
        buf: &mut rust_alloc::vec::Vec<u8>,
        reader: &mut S,
    ) -> anyhow::Result<Option<Self>>
    where
        S: ?Sized + Unpin + AsyncBufRead,
    {
        let mut headers = Headers::default();

        loop {
            buf.clear();

            let len = match reader.read_until(b'\n', buf).await {
                Ok(len) => len,
                Err(error) => return Err(error.into()),
            };

            if len == 0 {
                return Ok(None);
            }

            debug_assert!(len == buf.len());
            let buf = &buf[..len];

            let buf = std::str::from_utf8(buf)?;
            let line = buf.trim();

            if line.is_empty() {
                break;
            }

            let Some((key, value)) = line.split_once(':') else {
                return Err(anyhow!("bad header"));
            };

            let key = key.trim();
            let value = value.trim();
            let key = key.to_lowercase();

            match key.as_str() {
                "content-type" => match value {
                    "application/vscode-jsonrpc; charset=utf-8" => {
                        headers.content_type = Some(ContentType::JsonRPC);
                    }
                    value => {
                        return Err(anyhow!("bad value: {:?}", value));
                    }
                },
                "content-length" => {
                    let value = value
                        .parse::<u32>()
                        .map_err(|e| anyhow!("bad content-length: {}: {}", value, e))?;

                    headers.content_length = Some(value);
                }
                key => {
                    return Err(anyhow!("header not supported: {:?}", key));
                }
            }
        }

        Ok(Some(headers))
    }
}
