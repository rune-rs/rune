use crate::future::SelectFuture;
use crate::{Future, Mut, Value, VmError};
use futures_core::Stream;
use futures_util::stream::FuturesUnordered;
use std::future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// A stored select.
#[derive(Debug)]
pub struct Select {
    futures: FuturesUnordered<SelectFuture<usize, Mut<Future>>>,
}

impl Select {
    /// Construct a new stored select.
    pub(crate) fn new(futures: FuturesUnordered<SelectFuture<usize, Mut<Future>>>) -> Self {
        Self { futures }
    }
}

impl future::Future for Select {
    type Output = Result<(usize, Value), VmError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let poll = Pin::new(&mut self.futures).poll_next(cx);

        let poll = match poll {
            Poll::Ready(poll) => poll.expect("inner stream should never end"),
            Poll::Pending => return Poll::Pending,
        };

        Poll::Ready(poll)
    }
}
