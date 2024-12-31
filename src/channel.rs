use crate::error::SendError;
use crate::metrics::ChannelMetrics;
use async_trait::async_trait;
use futures::Sink;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc;
use tracing::{debug, error, instrument, span, Instrument, Level};

/// A sender handle to a channel
#[derive(Debug, Clone)]
pub struct Sender<T> {
    inner: mpsc::Sender<T>,
    gauge: prometheus::IntGauge,
    total_messages: Option<prometheus::IntCounter>,
}

/// A receiver handle to a channel
#[derive(Debug)]
pub struct Receiver<T> {
    inner: mpsc::Receiver<T>,
    gauge: prometheus::IntGauge,
    total_messages: Option<prometheus::IntCounter>,
}

/// A permit for sending a value
pub struct Permit<'a, T> {
    sender: &'a Sender<T>,
    _permit: mpsc::Permit<'a, T>,
}

impl<T> Permit<'_, T> {
    /// Send a value using this permit
    pub fn send(self, value: T) {
        self._permit.send(value);
        self.sender.gauge.inc();
        if let Some(ref counter) = self.sender.total_messages {
            counter.inc();
        }
    }
}

/// Creates a new channel with the given buffer size and metrics
pub fn channel<T>(buffer: usize, metrics: ChannelMetrics) -> (Sender<T>, Receiver<T>) {
    let (tx, rx) = mpsc::channel(buffer);
    let gauge = metrics.queue_size;
    let total_messages = metrics.total_messages;

    (
        Sender {
            inner: tx,
            gauge: gauge.clone(),
            total_messages: total_messages.clone(),
        },
        Receiver {
            inner: rx,
            gauge,
            total_messages,
        },
    )
}

/// Creates a new channel with total message counting
pub fn channel_with_total<T>(
    buffer: usize,
    gauge: &prometheus::IntGauge,
    total: &prometheus::IntCounter,
) -> (Sender<T>, Receiver<T>) {
    let (tx, rx) = mpsc::channel(buffer);

    (
        Sender {
            inner: tx,
            gauge: gauge.clone(),
            total_messages: Some(total.clone()),
        },
        Receiver {
            inner: rx,
            gauge: gauge.clone(),
            total_messages: Some(total.clone()),
        },
    )
}

impl<T> Sender<T> {
    /// Try to send a value without waiting for capacity
    pub fn try_send(&self, value: T) -> Result<(), SendError<T>> {
        match self.inner.try_send(value) {
            Ok(()) => {
                self.gauge.inc();
                if let Some(ref counter) = self.total_messages {
                    counter.inc();
                }
                Ok(())
            }
            Err(err) => Err(err.into()),
        }
    }

    /// Send a value, waiting for capacity if needed
    #[instrument(skip(self, value), level = "debug")]
    pub async fn send(&self, value: T) -> Result<(), SendError<T>> {
        debug!("attempting to send value");
        match self.inner.send(value).await {
            Ok(()) => {
                self.gauge.inc();
                if let Some(ref counter) = self.total_messages {
                    counter.inc();
                }
                debug!("value sent successfully");
                Ok(())
            }
            Err(err) => {
                error!(?err, "failed to send value");
                Err(err.into())
            }
        }
    }

    /// Returns true if the channel has been closed
    pub fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }
}

impl<T> Receiver<T> {
    /// Receive the next value
    #[instrument(skip(self), level = "debug")]
    pub async fn recv(&mut self) -> Option<T> {
        debug!("waiting to receive value");
        let msg = self.inner.recv().await;
        if msg.is_some() {
            self.gauge.dec();
            debug!("value received successfully");
        } else {
            debug!("channel closed, no more values");
        }
        msg
    }

    /// Try to receive a value without waiting
    pub fn try_recv(&mut self) -> Result<T, mpsc::error::TryRecvError> {
        match self.inner.try_recv() {
            Ok(msg) => {
                self.gauge.dec();
                Ok(msg)
            }
            Err(e) => Err(e),
        }
    }

    /// Close the channel
    pub fn close(&mut self) {
        self.inner.close()
    }

    /// Get the total messages counter if enabled
    pub fn total_messages(&self) -> Option<&prometheus::IntCounter> {
        self.total_messages.as_ref()
    }
}

/// Trait for types that support permit-based sending
#[async_trait]
pub trait WithPermit<T>: Send + Sync {
    /// Reserve capacity to send a value
    async fn reserve(&self) -> Result<Permit<'_, T>, SendError<()>>;

    /// Wait for a permit and a future to complete
    async fn with_permit<F>(&self, future: F) -> Result<(Permit<'_, T>, F::Output), SendError<()>>
    where
        F: Future + Send,
        F::Output: Send;
}

#[async_trait]
impl<T: Send> WithPermit<T> for Sender<T> {
    async fn reserve(&self) -> Result<Permit<'_, T>, SendError<()>> {
        match self.inner.reserve().await {
            Ok(permit) => Ok(Permit {
                sender: self,
                _permit: permit,
            }),
            Err(_) => Err(SendError::Closed(())),
        }
    }

    #[instrument(skip(self, future), level = "debug")]
    async fn with_permit<F>(&self, future: F) -> Result<(Permit<'_, T>, F::Output), SendError<()>>
    where
        F: Future + Send,
        F::Output: Send,
    {
        // First get a permit to ensure we have capacity
        debug!("requesting permit");
        let permit = self.reserve().await?;
        debug!("permit acquired");

        // Then execute the future - this ensures we don't lose the future's result
        // if the permit acquisition is cancelled
        let output = future
            .instrument(span!(Level::DEBUG, "permit_future"))
            .await;

        debug!("future completed with permit");
        Ok((permit, output))
    }
}

impl<T> Sink<T> for Sender<T> {
    type Error = SendError<T>;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        self.try_send(item)
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}
