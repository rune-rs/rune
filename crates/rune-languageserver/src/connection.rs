use crate::envelope;
use anyhow::{anyhow, bail, Result};
use std::fmt;
use std::sync::Arc;
use tokio::io;
use tokio::io::{
    AsyncBufRead, AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader,
};
use tokio::sync::Mutex;

/// An input frame.
#[derive(Debug)]
pub struct Frame<'a> {
    pub content: &'a [u8],
}

/// Input connection.
pub struct Input {
    buf: Vec<u8>,
    stdin: BufReader<io::Stdin>,
}

impl Input {
    /// Get the next input frame.
    pub async fn next(&mut self) -> Result<Option<Frame<'_>>> {
        let headers = match Headers::read(&mut self.buf, &mut self.stdin).await? {
            Some(headers) => headers,
            None => return Ok(None),
        };

        log::trace!("headers: {:?}", headers);

        let length = match headers.content_length {
            Some(length) => length as usize,
            None => bail!("missing content-length"),
        };

        self.buf.resize(length, 0u8);

        log::trace!("read frame: {}", self.buf.len());
        self.stdin.read_exact(&mut self.buf[..]).await?;

        Ok(Some(Frame { content: &self.buf }))
    }
}

/// Output connection.
#[derive(Clone)]
pub struct Output {
    stdout: Arc<Mutex<io::Stdout>>,
}

impl Output {
    /// Send the given response.
    pub async fn response<R>(&self, id: Option<envelope::RequestId>, result: R) -> Result<()>
    where
        R: serde::Serialize,
    {
        let response = envelope::ResponseMessage {
            jsonrpc: crate::envelope::V2,
            id,
            result: Some(result),
            error: None::<envelope::ResponseError<()>>,
        };

        let mut bytes = serde_json::to_vec(&response)?;
        self.write_response(&mut bytes).await?;
        Ok(())
    }

    /// Send the given error as response.
    pub async fn error<D, M>(
        &self,
        id: Option<envelope::RequestId>,
        code: envelope::Code,
        message: M,
        data: Option<D>,
    ) -> Result<()>
    where
        D: serde::Serialize,
        M: fmt::Display,
    {
        let response = envelope::ResponseMessage {
            jsonrpc: crate::envelope::V2,
            id,
            result: None::<()>,
            error: Some(envelope::ResponseError {
                code,
                message: message.to_string(),
                data,
            }),
        };

        let mut bytes = serde_json::to_vec(&response)?;
        self.write_response(&mut bytes).await?;
        Ok(())
    }

    /// Send the given notification
    pub async fn notification<N>(&self, notification: N::Params) -> Result<()>
    where
        N: lsp::notification::Notification,
    {
        let notification = crate::envelope::NotificationMessage {
            jsonrpc: crate::envelope::V2,
            method: N::METHOD,
            params: notification,
        };

        let mut bytes = serde_json::to_vec(&notification)?;
        self.write_response(&mut bytes).await?;
        Ok(())
    }

    /// Send a log message.
    pub async fn log<M>(&self, typ: lsp::MessageType, message: M) -> Result<()>
    where
        M: fmt::Display,
    {
        self.notification::<lsp::notification::LogMessage>(lsp::LogMessageParams {
            typ,
            message: message.to_string(),
        })
        .await?;

        Ok(())
    }

    /// Write the given response body.
    async fn write_response(&self, bytes: &mut Vec<u8>) -> Result<()> {
        use std::io::Write as _;

        let mut m = Vec::new();

        write!(m, "Content-Length: {}\r\n", bytes.len())?;
        write!(m, "\r\n")?;
        m.append(bytes);

        let mut stdout = self.stdout.lock().await;
        stdout.write_all(&m).await?;
        stdout.flush().await?;
        Ok(())
    }
}

/// Setup a stdin/stdout connection.
pub fn stdio() -> Result<(Input, Output)> {
    let stdin = io::stdin();
    let stdout = io::stdout();

    let input = Input {
        buf: Vec::new(),
        stdin: BufReader::new(stdin),
    };

    let output = Output {
        stdout: Arc::new(Mutex::new(stdout)),
    };

    Ok((input, output))
}

#[derive(Debug)]
pub enum ContentType {
    JsonRPC,
}

#[derive(Default, Debug)]
pub struct Headers {
    pub content_length: Option<u32>,
    pub content_type: Option<ContentType>,
}

impl Headers {
    /// Read headers from the given line stream.
    pub async fn read<S>(buf: &mut Vec<u8>, reader: &mut S) -> anyhow::Result<Option<Self>>
    where
        S: Unpin + AsyncBufRead,
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

            let mut parts = line.splitn(2, ':').map(str::trim);

            let (key, value) = match (parts.next(), parts.next()) {
                (Some(key), Some(value)) => (key, value),
                out => {
                    return Err(anyhow!("bad header: {:?}", out));
                }
            };

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
                    return Err(anyhow!("unsupported header: {:?}", key));
                }
            }
        }

        Ok(Some(headers))
    }
}
