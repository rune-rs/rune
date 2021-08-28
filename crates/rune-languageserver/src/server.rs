use crate::connection::Output;
use crate::envelope::{Code, IncomingMessage};
use crate::State;
use anyhow::Result;
use hashbrown::HashMap;
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
    handlers: HashMap<&'static str, Box<Handler>>,
}

impl Server {
    /// Construct a new server implementation associated with the given output.
    pub fn new(
        output: Output,
        rebuild_tx: mpsc::Sender<()>,
        context: runestick::Context,
        options: rune::Options,
    ) -> Self {
        Self {
            state: State::new(rebuild_tx, context, options),
            output,
            handlers: HashMap::new(),
        }
    }

    /// Get a clone of the server output.
    pub fn output(&self) -> Output {
        self.output.clone()
    }

    /// Rebuild the projects.
    pub async fn rebuild(&self) -> Result<()> {
        self.state.rebuild(&self.output).await?;
        Ok(())
    }

    /// Process an incoming message.
    pub async fn process(&self, mut incoming: IncomingMessage) -> Result<()> {
        use lsp::request::Request as _;

        let method = std::mem::take(&mut incoming.method);

        // If server is not initialized, reject incoming requests.
        if !self.state.is_initialized() && method != lsp::request::Initialize::METHOD {
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

        if let Some(handler) = self.handlers.get(&method.as_str()) {
            handler(self.state.clone(), self.output.clone(), incoming).await?;
            return Ok(());
        }

        log::warn!("Unhandled method `{}`", method);

        self.output
            .log(
                lsp::MessageType::Info,
                format!("Unhandled method `{}`", method),
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

        self.handlers.insert(T::METHOD, handler);
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

        self.handlers.insert(T::METHOD, handler);
    }
}
