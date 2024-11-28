use crate::error::SendError;
use crate::metrics::ChannelMetrics;
use tokio::sync::watch;
use std::sync::Arc;
use tracing::{debug, error, instrument};

/// A sender for the watch channel.
/// 
/// The watch channel allows watching for value changes and supports
/// multiple receivers that can independently track value updates.
///
/// # Examples
///
/// ```rust
/// use tokio_prometheus_channel_backpressure::{watch_channel, ChannelMetrics};
/// use prometheus::Registry;
///
/// #[tokio::main]
/// async fn main() {
///     let registry = Registry::new();
///     let metrics = ChannelMetrics::new_basic("example", "watch example", &registry).unwrap();
///     
///     // Create a channel with initial value 0
///     let (tx, mut rx1) = watch_channel(0, metrics);
///     let mut rx2 = rx1.clone();
///     
///     // Send updates
///     tx.send(42).unwrap();
///     
///     // Both receivers can see the new value
///     rx1.changed().await.unwrap();
///     rx2.changed().await.unwrap();
///     
///     assert_eq!(*rx1.borrow(), 42);
///     assert_eq!(*rx2.borrow(), 42);
/// }
/// ```
#[derive(Debug)]
pub struct Sender<T> {
    inner: watch::Sender<T>,
    gauge: prometheus::IntGauge,
    total_messages: Option<prometheus::IntCounter>,
}

/// A receiver for the watch channel
#[derive(Debug, Clone)]
pub struct Receiver<T> {
    inner: watch::Receiver<T>,
    gauge: Arc<prometheus::IntGauge>,
    total_messages: Option<prometheus::IntCounter>,
}

/// Creates a new watch channel with an initial value and metrics
pub fn channel<T>(
    initial: T,
    metrics: ChannelMetrics,
) -> (Sender<T>, Receiver<T>) {
    let (tx, rx) = watch::channel(initial);
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

impl<T> Sender<T> {
    /// Send a value, replacing the current value
    #[instrument(skip(self, value), level = "debug")]
    pub fn send(&self, value: T) -> Result<(), SendError<T>> {
        debug!("attempting to update watch value");
        match self.inner.send(value) {
            Ok(()) => {
                self.gauge.inc();
                if let Some(ref counter) = self.total_messages {
                    counter.inc();
                }
                debug!("watch value updated successfully");
                Ok(())
            }
            Err(e) => {
                error!("failed to update watch value");
                Err(SendError::Closed(e.0))
            },
        }
    }

    /// Get number of receivers
    pub fn receiver_count(&self) -> usize {
        self.inner.receiver_count()
    }
}

impl<T: Clone> Receiver<T> {
    /// Get the current value
    pub fn borrow(&self) -> watch::Ref<'_, T> {
        self.inner.borrow()
    }

    /// Wait for the value to change
    pub async fn changed(&mut self) -> Result<(), watch::error::RecvError> {
        let result = self.inner.changed().await;
        if result.is_ok() {
            self.gauge.dec();
            if let Some(ref counter) = self.total_messages {
                counter.inc();
            }
        }
        result
    }

    /// Returns true if the sender has been dropped
    pub fn has_changed(&self) -> bool {
        self.inner.has_changed().unwrap_or(false)
    }
}
