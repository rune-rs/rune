use crate::connection::Output;
use crate::envelope::{Code, IncomingMessage};
use crate::State;
use anyhow::Result;
use bstr::BStr;
use hashbrown::HashMap;
use rune::{Context, Options};
use std::future::Future;
use std::pin::Pin;
use tokio::sync::mpsc;

/// A boxed future returned from a handler.
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + 'static>>;

/// A raw message handler.
type Handler = dyn Fn(State, Output, IncomingMessage) -> BoxFuture<Result<()>> + 'static;

/// An lsp server implementation.
pub struct Server {
    /// Shared server state.
    state: State,
    /// The output abstraction.
    output: Output,
    /// Handlers registered in the server.
    handlers: HashMap<&'static BStr, Box<Handler>>,
}

impl Server {
    /// Construct a new server implementation associated with the given output.
    pub fn new(
        output: Output,
        rebuild_tx: mpsc::Sender<()>,
        context: Context,
        options: Options,
    ) -> Self {
        Self {
            state: State::new(rebuild_tx, context, options),
            output,
            handlers: HashMap::new(),
        }
    }

    /// Get a reference to server output.
    pub fn output(&self) -> &Output {
        &self.output
    }

    /// Get a reference to server state.
    pub fn state(&self) -> &State {
        &self.state
    }

    /// Rebuild the projects.
    pub async fn rebuild(&self) -> Result<()> {
        self.state.rebuild(&self.output).await?;
        Ok(())
    }

    /// Process an incoming message.
    pub async fn process(&self, incoming: IncomingMessage<'_>) -> Result<()> {
        use lsp::request::Request as _;

        // If server is not initialized, reject incoming requests.
        if !self.state.is_initialized() && incoming.method != lsp::request::Initialize::METHOD {
            self.output
                .error(
                    incoming.id,
                    Code::InvalidRequest,
                    "Server not initialized",
                    None::<()>,
                )
                .await?;

            return Ok(());
        }

        if let Some(handler) = self.handlers.get(incoming.method) {
            handler(self.state.clone(), self.output.clone(), incoming).await?;
            return Ok(());
        }

        tracing::warn!("Unhandled method `{}`", incoming.method);

        self.output
            .log(
                lsp::MessageType::INFO,
                format!("Unhandled method `{}`", incoming.method),
            )
            .await?;

        Ok(())
    }

    /// Register a request handler.
    pub fn request_handler<T, H, O>(&mut self, handler: H)
    where
        T: lsp::request::Request,
        H: 'static + Copy + Fn(State, Output, T::Params) -> O,
        O: 'static + Future<Output = Result<T::Result>>,
    {
        let handler: Box<Handler> = Box::new(move |state, output, incoming| {
            Box::pin(async move {
                use serde::de::Deserialize as _;
                let params = <T::Params>::deserialize(incoming.params)?;
                let result = handler(state, output.clone(), params).await?;
                output.response(incoming.id, result).await?;
                Ok(())
            })
        });

        self.handlers.insert(BStr::new(T::METHOD), handler);
    }

    /// Register a notification handler.
    pub fn notification_handler<T, H, O>(&mut self, handler: H)
    where
        T: lsp::notification::Notification,
        H: 'static + Copy + Fn(State, Output, T::Params) -> O,
        O: 'static + Future<Output = Result<()>>,
    {
        let handler: Box<Handler> = Box::new(move |state, output, incoming| {
            use serde::de::Deserialize as _;

            Box::pin(async move {
                let params = <T::Params>::deserialize(incoming.params)?;
                handler(state, output, params).await
            })
        });

        self.handlers.insert(BStr::new(T::METHOD), handler);
    }
}
