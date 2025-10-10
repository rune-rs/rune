use core::fmt;
use core::str;

use anyhow::{anyhow, bail, Context as _, Result};
use tokio::io::{AsyncBufRead, AsyncBufReadExt as _, AsyncRead, AsyncReadExt as _, BufReader};

use crate::alloc::prelude::*;
use crate::languageserver::envelope;

enum State {
    /// Initial state, before header has been received.
    Initial,
    /// Reading state, when a header has been received.
    Reading(usize),
}

/// Input connection.
pub(super) struct Input<I> {
    reader: BufReader<I>,
    state: State,
}

impl<I> Input<I>
where
    I: Unpin + AsyncRead,
{
    /// Create a new input connection.
    pub(super) fn new(reader: I) -> Self {
        Self {
            reader: BufReader::new(reader),
            state: State::Initial,
        }
    }

    /// Get the next input frame.
    pub(super) async fn next(&mut self, buf: &mut rust_alloc::vec::Vec<u8>) -> Result<bool> {
        loop {
            match self.state {
                State::Initial => {
                    let Some(headers) = Headers::read(buf, &mut self.reader).await? else {
                        return Ok(false);
                    };

                    tracing::trace!(?headers, "Received headers");

                    let len = match headers.content_length {
                        Some(length) => length as usize,
                        None => bail!("Missing content-length in header"),
                    };

                    buf.resize(len, 0u8);
                    self.state = State::Reading(0);
                }
                State::Reading(ref mut at) => {
                    let n = self.reader.read(&mut buf[*at..]).await?;

                    *at += n;

                    if *at == buf.len() {
                        self.state = State::Initial;
                        return Ok(true);
                    }

                    if n == 0 {
                        return Ok(false);
                    }
                }
            }
        }
    }
}

/// Buffer for outbound data.
pub(super) struct Outbound {
    scratch: rust_alloc::vec::Vec<u8>,
    buf: rust_alloc::vec::Vec<u8>,
    write: usize,
}

impl Outbound {
    pub(super) fn new() -> Self {
        Self {
            scratch: rust_alloc::vec::Vec::new(),
            buf: rust_alloc::vec::Vec::new(),
            write: 0,
        }
    }

    /// Check if the buffer is empty.
    pub(super) fn is_empty(&self) -> bool {
        self.write >= self.buf.len()
    }

    /// Get slice of readable data.
    pub(super) fn readable(&self) -> &[u8] {
        self.buf.get(self.write..).unwrap_or_default()
    }

    /// Advance the write position by the given amount.
    pub(super) fn advance(&mut self, n: usize) {
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

#[derive(Debug, Clone, Copy)]
pub(super) enum ContentType {
    JsonRPC,
}

#[derive(Default, Debug, Clone, Copy)]
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
        let mut any = false;

        loop {
            let len = match reader.read_until(b'\n', buf).await {
                Ok(len) => len,
                Err(error) => return Err(error.into()),
            };

            if len == 0 {
                return Ok(None);
            }

            debug_assert_eq!(len, buf.len());

            let line = buf.get(..len).unwrap_or_default();
            let line = str::from_utf8(line).context("decoding line")?;
            let line = line.trim();

            if line.is_empty() {
                break;
            }

            let Some((key, value)) = line.split_once(':') else {
                return Err(anyhow!("bad header"));
            };

            let key = key.trim();
            let value = value.trim();

            'done: {
                if key.eq_ignore_ascii_case("content-type") {
                    match value {
                        "application/vscode-jsonrpc; charset=utf-8" => {
                            headers.content_type = Some(ContentType::JsonRPC);
                        }
                        value => {
                            return Err(anyhow!("Unsupported content-type `{value}`"));
                        }
                    }

                    any = true;
                    break 'done;
                }

                if key.eq_ignore_ascii_case("content-length") {
                    let value = value
                        .parse::<u32>()
                        .map_err(|e| anyhow!("bad content-length: {}: {}", value, e))?;

                    headers.content_length = Some(value);
                    any = true;
                    break 'done;
                };

                bail!("Unsupported header `{key}`");
            }

            buf.clear();
        }

        if !any {
            return Ok(None);
        }

        Ok(Some(headers))
    }
}
