use crate::{mpsc_channel, mpsc_channel_with_total, ChannelMetrics, WithPermit};
use futures::task::{noop_waker, Context, Poll};
use futures::{stream::FuturesUnordered, FutureExt, StreamExt};
use prometheus::Registry;
use std::time::Duration;
use tracing_subscriber::fmt::format::FmtSpan;

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("debug")
        .with_span_events(FmtSpan::CLOSE)
        .try_init();
}

#[tokio::test]
async fn test_basic_send_recv() {
    init_tracing();
    let registry = Registry::new();
    let metrics = ChannelMetrics::new_basic("test", "test channel", &registry).unwrap();

    let (tx, mut rx) = mpsc_channel::<i32>(2, metrics);

    tx.send(1).await.unwrap();
    let val = rx.recv().await.unwrap();
    assert_eq!(val, 1);
}

#[tokio::test]
async fn test_total_messages() {
    let registry = Registry::new();
    let metrics = ChannelMetrics::new("test", "test channel", &registry).unwrap();

    let (tx, mut rx) = mpsc_channel_with_total(
        2,
        &metrics.queue_size,
        metrics.total_messages.as_ref().unwrap(),
    );

    tx.send(1).await.unwrap();
    tx.send(2).await.unwrap();

    rx.recv().await.unwrap();
    rx.recv().await.unwrap();
    assert_eq!(rx.total_messages().unwrap().get(), 2);
}

#[tokio::test]
async fn test_empty_closed_channel() {
    let registry = Registry::new();
    let metrics = ChannelMetrics::new_basic("test_empty", "test empty channel", &registry).unwrap();

    let (tx, mut rx) = mpsc_channel::<i32>(8, metrics);

    tx.send(42).await.unwrap();
    let received_item = rx.recv().await.unwrap();
    assert_eq!(received_item, 42);

    // Close channel and verify behavior
    rx.close();
    assert!(tx.is_closed());
    let send_result = tx.send(1).await;
    assert!(send_result.is_err());
}

#[tokio::test]
async fn test_send_backpressure() {
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);

    let registry = Registry::new();
    let metrics =
        ChannelMetrics::new_basic("test_backpressure", "test backpressure", &registry).unwrap();

    let (tx, mut rx) = mpsc_channel::<i32>(1, metrics);

    // Fill the channel
    tx.send(1).await.unwrap();

    // Try to send when full
    let mut send_fut = Box::pin(tx.send(2));
    assert!(matches!(send_fut.poll_unpin(&mut cx), Poll::Pending));

    // Receive to make space
    let item = rx.recv().await.unwrap();
    assert_eq!(item, 1);

    // Now the send should complete
    assert!(send_fut.now_or_never().is_some());
}

#[tokio::test]
async fn test_send_backpressure_multi_senders() {
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);

    let registry = Registry::new();
    let metrics =
        ChannelMetrics::new_basic("test_multi", "test multiple senders", &registry).unwrap();

    let (tx1, mut rx) = mpsc_channel::<i32>(1, metrics);
    let tx2 = tx1.clone();

    // First sender fills the channel
    tx1.send(1).await.unwrap();

    // Second sender tries to send
    let mut send_fut = Box::pin(tx2.send(2));
    assert!(matches!(send_fut.poll_unpin(&mut cx), Poll::Pending));

    // Receive to make space
    let item = rx.recv().await.unwrap();
    assert_eq!(item, 1);

    // Now the second send should complete
    assert!(send_fut.now_or_never().is_some());
}

#[tokio::test]
async fn test_reserve_backpressure() {
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);

    let registry = Registry::new();
    let metrics =
        ChannelMetrics::new_basic("test_reserve", "test reserve backpressure", &registry).unwrap();

    let (tx, mut rx) = mpsc_channel::<i32>(1, metrics);

    // Get first permit
    let permit = tx.reserve().await.unwrap();

    // Try to get another permit
    let mut reserve_fut = Box::pin(tx.reserve());
    assert!(matches!(reserve_fut.poll_unpin(&mut cx), Poll::Pending));

    // Send with first permit
    permit.send(1);

    // Receive to make space
    let item = rx.recv().await.unwrap();
    assert_eq!(item, 1);

    // Now the reserve should complete
    assert!(reserve_fut.now_or_never().is_some());
}

#[tokio::test]
async fn test_reserve_and_drop() {
    let registry = Registry::new();
    let metrics =
        ChannelMetrics::new_basic("test_reserve_drop", "test reserve and drop", &registry).unwrap();

    let (tx, _rx) = mpsc_channel::<i32>(8, metrics);

    let permit = tx.reserve().await.unwrap();
    drop(permit);
}

#[tokio::test]
async fn test_with_permit_cancel_safety() {
    let registry = Registry::new();
    let metrics =
        ChannelMetrics::new_basic("test_cancel", "test cancel safety", &registry).unwrap();

    let (tx_in, mut rx_in) = mpsc_channel(100, metrics.clone()); // Input channel with mutability
    let (tx_out, mut rx_out) = mpsc_channel(1, metrics); // Output channel with backpressure

    // Spawn processor task
    tokio::spawn(async move {
        let mut waiting = FuturesUnordered::new();

        loop {
            tokio::select! {
                Some(input) = rx_in.recv() => {
                    waiting.push(std::future::ready(input));
                }

                result = tx_out.with_permit(waiting.next()) => {
                    match result {
                        Ok((permit, Some(value))) => permit.send(value),
                        Err(_) => break, // Exit loop on channel close
                        _ => continue,
                    }
                }
            }
        }
    });

    // Fill input channel
    for i in 0..100 {
        tx_in.send(i).await.unwrap();
    }

    // Verify we can receive all values despite cancellations
    let mut received = vec![];
    while let Ok(Some(val)) = tokio::time::timeout(Duration::from_secs(1), rx_out.recv()).await {
        received.push(val);
    }

    assert_eq!(received, (0..100).collect::<Vec<_>>());
}
