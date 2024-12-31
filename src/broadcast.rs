use crate::error::SendError;
use crate::metrics::ChannelMetrics;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, instrument};

/// A broadcast channel sender that integrates with Prometheus metrics.
///
/// The broadcast channel allows sending messages to multiple receivers.
/// Each receiver gets a copy of all messages sent after they subscribed.
///
/// # Examples
///
/// ```rust
/// use tokio_prometheus_metered_channel::{broadcast_channel, ChannelMetrics};
/// use prometheus::Registry;
///
/// #[tokio::main]
/// async fn main() {
///     let registry = Registry::new();
///     let metrics = ChannelMetrics::new_basic("example", "broadcast example", &registry).unwrap();
///     
///     let (tx, mut rx1) = broadcast_channel(10, metrics);
///     let mut rx2 = tx.subscribe();
///     
///     // Send a message to all receivers
///     tx.send(42).unwrap();
///     
///     // Both receivers get the message
///     assert_eq!(rx1.recv().await.unwrap(), 42);
///     assert_eq!(rx2.recv().await.unwrap(), 42);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Sender<T: Clone> {
    inner: broadcast::Sender<T>,
    gauge: prometheus::IntGauge,
    total_messages: Option<prometheus::IntCounter>,
}

/// A receiver for the broadcast channel
#[derive(Debug)]
pub struct Receiver<T: Clone> {
    inner: broadcast::Receiver<T>,
    gauge: Arc<prometheus::IntGauge>,
    total_messages: Option<prometheus::IntCounter>,
}

/// Creates a new broadcast channel with given capacity and metrics
pub fn channel<T: Clone>(capacity: usize, metrics: ChannelMetrics) -> (Sender<T>, Receiver<T>) {
    let (tx, rx) = broadcast::channel(capacity);
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
            gauge: Arc::new(gauge),
            total_messages,
        },
    )
}

impl<T: Clone> Sender<T> {
    /// Send a value to all receivers
    #[instrument(skip(self, value), level = "debug")]
    pub fn send(&self, value: T) -> Result<(), SendError<T>> {
        debug!("attempting to broadcast value");
        match self.inner.send(value) {
            Ok(_) => {
                self.gauge.inc();
                if let Some(ref counter) = self.total_messages {
                    counter.inc();
                }
                debug!("value broadcasted successfully");
                Ok(())
            }
            Err(e) => {
                error!("failed to broadcast value");
                Err(SendError::Closed(e.0))
            }
        }
    }

    /// Create a new receiver for this broadcast channel
    pub fn subscribe(&self) -> Receiver<T> {
        Receiver {
            inner: self.inner.subscribe(),
            gauge: Arc::clone(&Arc::new(self.gauge.clone())),
            total_messages: self.total_messages.clone(),
        }
    }

    /// Get number of active receivers
    pub fn receiver_count(&self) -> usize {
        self.inner.receiver_count()
    }
}

impl<T: Clone> Receiver<T> {
    /// Receive the next value
    pub async fn recv(&mut self) -> Result<T, broadcast::error::RecvError> {
        match self.inner.recv().await {
            Ok(msg) => {
                self.gauge.dec();
                if let Some(ref counter) = self.total_messages {
                    counter.inc();
                }
                Ok(msg)
            }
            Err(e) => Err(e),
        }
    }

    /// Try to receive a value without waiting
    pub fn try_recv(&mut self) -> Result<T, broadcast::error::TryRecvError> {
        match self.inner.try_recv() {
            Ok(msg) => {
                self.gauge.dec();
                Ok(msg)
            }
            Err(e) => Err(e),
        }
    }

    /// Get the total messages counter if enabled
    pub fn total_messages(&self) -> Option<&prometheus::IntCounter> {
        self.total_messages.as_ref()
    }
}
