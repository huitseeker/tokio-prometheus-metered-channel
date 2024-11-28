//! Metered channels with Prometheus metrics integration.
//!
//! This crate provides channel implementations that combine Tokio's asynchronous channels
//! with Prometheus metrics integration. The channels support
//! proper backpressure through permit-based operations and comprehensive metrics tracking.
//!
//! # Features
//!
//! - Prometheus metrics integration for monitoring channel behavior
//! - Cancel-safe permit operations for reliable backpressure handling
//! - Multiple channel types (mpsc, broadcast, watch) with consistent interfaces
//! - Comprehensive error handling with detailed error types
//! - Full test coverage ensuring reliability
//!
//! # Channel Types
//!
//! - [`mpsc_channel`]: Multi-producer, single-consumer channel with metrics
//! - [`broadcast_channel`]: Multi-producer, multi-consumer broadcast channel
//! - [`watch_channel`]: Single-producer, multi-consumer watch channel
//!
//! # Example
//!
//! ```rust
//! use tokio_prometheus_channel_backpressure::{mpsc_channel, ChannelMetrics};
//! use prometheus::Registry;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Create a new registry and metrics
//!     let registry = Registry::new();
//!     let metrics = ChannelMetrics::new_basic("example", "example channel", &registry).unwrap();
//!     
//!     // Create a channel with capacity 10
//!     let (tx, mut rx) = mpsc_channel(10, metrics);
//!     
//!     // Send a value
//!     tx.send(42).await.unwrap();
//!     
//!     // Receive the value
//!     let value = rx.recv().await.unwrap();
//!     assert_eq!(value, 42);
//! }
//! ```
//!
//! # Credits
//!
//! This implementation is inspired by and builds upon work from:
//! - Mysten Labs' Narwhal project
//! - Diem's channel implementations

#![warn(missing_docs)]

/// Broadcast channel implementation with prometheus metrics integration.
///
/// This channel type allows sending messages to multiple receivers.
/// Each receiver gets a copy of each message sent after they subscribed.
pub mod broadcast;

mod channel;
mod error;
mod metrics;

/// Watch channel implementation with prometheus metrics integration.
///
/// This channel type allows watching for value changes.
/// New receivers see the latest value and all subsequent changes.
pub mod watch;

#[cfg(test)]
mod tests;

// Re-export specific items from channel module
pub use channel::{
    channel as mpsc_channel, channel_with_total as mpsc_channel_with_total,
    Receiver as MpscReceiver, Sender as MpscSender, WithPermit,
};

pub use broadcast::channel as broadcast_channel;
pub use error::SendError;
pub use metrics::ChannelMetrics;
pub use watch::channel as watch_channel;

/// Re-exports of commonly used types
pub mod prelude {
    pub use crate::{
        broadcast::channel as broadcast_channel, mpsc_channel, mpsc_channel_with_total,
        watch::channel as watch_channel, ChannelMetrics, MpscReceiver, MpscSender, SendError,
        WithPermit,
    };
}
