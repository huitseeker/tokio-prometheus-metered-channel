use crate::broadcast_channel;
use crate::ChannelMetrics;
use prometheus::Registry;
use tokio::sync::broadcast::error::{RecvError, TryRecvError};
use tracing_subscriber::fmt::format::FmtSpan;

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("debug")
        .with_span_events(FmtSpan::CLOSE)
        .try_init();
}

#[tokio::test]
async fn test_broadcast_basic() {
    init_tracing();
    let registry = Registry::new();
    let metrics = ChannelMetrics::new_basic("test_broadcast", "test broadcast channel", &registry).unwrap();
    
    let (tx, mut rx1) = broadcast_channel(10, metrics);
    let mut rx2 = tx.subscribe();
    
    // Send to multiple receivers
    tx.send(1).unwrap();
    
    let val1 = rx1.recv().await.unwrap();
    let val2 = rx2.recv().await.unwrap();
    
    assert_eq!(val1, 1);
    assert_eq!(val2, 1);
}

#[tokio::test]
async fn test_broadcast_metrics() {
    let registry = Registry::new();
    let metrics = ChannelMetrics::new("test_broadcast_metrics", "test broadcast metrics", &registry).unwrap();
    
    let (tx, mut rx) = broadcast_channel::<i32>(2, metrics);
    
    tx.send(1).unwrap();
    tx.send(2).unwrap();
    
    rx.recv().await.unwrap();
    rx.recv().await.unwrap();
    assert_eq!(rx.total_messages().unwrap().get(), 4);
}

#[tokio::test]
async fn test_broadcast_lagged() {
    let registry = Registry::new();
    let metrics = ChannelMetrics::new_basic("test_lag", "test lag", &registry).unwrap();
    
    let (tx, mut rx) = broadcast_channel(2, metrics);
    
    tx.send(1).unwrap();
    tx.send(2).unwrap();
    tx.send(3).unwrap(); // This will cause lag for slow receiver
    
    assert!(matches!(rx.recv().await, Err(RecvError::Lagged(_))));
}

#[tokio::test]
async fn test_broadcast_closed() {
    let registry = Registry::new();
    let metrics = ChannelMetrics::new_basic("test_closed", "test closed", &registry).unwrap();
    
    let (tx, mut rx) = broadcast_channel::<i32>(2, metrics);
    drop(tx);
    
    assert!(matches!(rx.recv().await, Err(RecvError::Closed)));
}

#[tokio::test]
async fn test_broadcast_try_recv() {
    let registry = Registry::new();
    let metrics = ChannelMetrics::new_basic("test_try", "test try recv", &registry).unwrap();
    
    let (tx, mut rx) = broadcast_channel(2, metrics);
    
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
    
    tx.send(1).unwrap();
    assert_eq!(rx.try_recv().unwrap(), 1);
    
    drop(tx);
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Closed)));
}

#[tokio::test]
async fn test_broadcast_multiple_subscribers() {
    let registry = Registry::new();
    let metrics = ChannelMetrics::new_basic("test_multi", "test multiple", &registry).unwrap();
    
    let (tx, _rx) = broadcast_channel(10, metrics);
    
    // Create multiple subscribers
    let mut rxs = vec![];
    for _ in 0..5 {
        rxs.push(tx.subscribe());
    }
    
    // All should receive the message
    tx.send(42).unwrap();
    
    for mut rx in rxs {
        assert_eq!(rx.recv().await.unwrap(), 42);
    }
    
    // only one active receiver left
    assert_eq!(tx.receiver_count(), 1);
}
