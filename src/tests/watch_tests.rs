use crate::watch_channel;
use crate::ChannelMetrics;
use prometheus::Registry;
use tracing_subscriber::fmt::format::FmtSpan;

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("debug")
        .with_span_events(FmtSpan::CLOSE)
        .try_init();
}

#[tokio::test]
async fn test_watch_channel() {
    init_tracing();
    let registry = Registry::new();
    let metrics = ChannelMetrics::new_basic("test_watch", "test watch channel", &registry).unwrap();
    
    let (tx, mut rx1) = watch_channel::channel(0, metrics);
    let mut rx2 = rx1.clone();
    
    // Update value
    tx.send(1).unwrap();
    
    // Both receivers should see the change
    rx1.changed().await.unwrap();
    rx2.changed().await.unwrap();
    
    assert_eq!(*rx1.borrow(), 1);
    assert_eq!(*rx2.borrow(), 1);
}

#[tokio::test]
async fn test_watch_metrics() {
    let registry = Registry::new();
    let metrics = ChannelMetrics::new("test_watch_metrics", "test watch metrics", &registry).unwrap();
    
    let (tx, mut rx) = watch_channel::channel(0, metrics);
    
    tx.send(1).unwrap();
    tx.send(2).unwrap();
    
    rx.changed().await.unwrap();
    rx.changed().await.unwrap();
    assert_eq!(rx.total_messages.as_ref().unwrap().get(), 2);
}
