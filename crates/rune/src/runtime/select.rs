use core::future;
use core::pin::Pin;
use core::task::{ready, Context, Poll};

use futures_core::Stream;
use futures_util::stream::FuturesUnordered;

use crate::runtime::future::SelectFuture;
use crate::runtime::{Future, Mut, Value, VmError};

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

    #[inline]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Some(result) = ready!(Pin::new(&mut self.futures).poll_next(cx)) else {
            return Poll::Ready(Err(VmError::panic("select: no futures to select from")));
        };

        Poll::Ready(result)
    }
}
