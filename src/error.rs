//! Error types for channel operations.
//! 
//! This module provides error types used throughout the crate for handling
//! various channel operation failures.

use std::fmt::Debug;
use tokio::sync::mpsc::error::{SendError as TokioSendError, TrySendError};

/// Error type for channel operations.
/// 
/// This type represents errors that can occur when sending messages through
/// any of the channel types provided by this crate.
///
/// # Examples
///
/// ```rust
/// use tokio_prometheus_metered_channel::{mpsc_channel, ChannelMetrics, SendError};
/// use prometheus::Registry;
///
/// #[tokio::main]
/// async fn main() {
///     let registry = Registry::new();
///     let metrics = ChannelMetrics::new_basic("example", "example channel", &registry).unwrap();
///     
///     let (tx, rx) = mpsc_channel(1, metrics);
///     drop(rx); // Close the channel
///     
///     // Trying to send on a closed channel
///     match tx.try_send(42) {
///         Err(SendError::Closed(val)) => println!("Channel closed, value: {}", val),
///         Err(SendError::Full(val)) => println!("Channel full, value: {}", val),
///         Ok(()) => println!("Send successful"),
///     }
/// }
/// ```
#[derive(Debug)]
pub enum SendError<T> {
    /// Channel is closed and cannot accept new messages.
    /// Contains the message that failed to send.
    Closed(T),
    /// Channel is at capacity and cannot accept new messages.
    /// Contains the message that failed to send.
    Full(T),
}

impl<T> From<TrySendError<T>> for SendError<T> {
    fn from(e: TrySendError<T>) -> Self {
        match e {
            TrySendError::Full(msg) => SendError::Full(msg),
            TrySendError::Closed(msg) => SendError::Closed(msg),
        }
    }
}

impl<T> From<TokioSendError<T>> for SendError<T> {
    fn from(e: TokioSendError<T>) -> Self {
        SendError::Closed(e.0)
    }
}

impl<T> std::error::Error for SendError<T> where T: Debug {}

impl<T> std::fmt::Display for SendError<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SendError::Closed(value) => write!(f, "send error: channel closed with value {:?}", value),
            SendError::Full(value) => write!(f, "send error: channel full with value {:?}", value),
        }
    }
}
