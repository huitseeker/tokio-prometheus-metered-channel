use crate::ChannelMetrics;
use prometheus::Registry;

#[test]
fn test_metrics_creation() {
    let registry = Registry::new();

    let metrics = ChannelMetrics::new("test_metrics", "test metrics", &registry).unwrap();

    assert_eq!(metrics.queue_size.get(), 0);
    assert_eq!(metrics.total_messages.unwrap().get(), 0);

    let basic_metrics =
        ChannelMetrics::new_basic("test_basic", "test basic metrics", &registry).unwrap();

    assert_eq!(basic_metrics.queue_size.get(), 0);
    assert!(basic_metrics.total_messages.is_none());
}

#[test]
fn test_metrics_clone() {
    let registry = Registry::new();
    let metrics = ChannelMetrics::new("test_clone", "test clone metrics", &registry).unwrap();

    let cloned = metrics.clone();

    metrics.queue_size.inc();
    assert_eq!(cloned.queue_size.get(), 1);

    metrics.total_messages.unwrap().inc();
    assert_eq!(cloned.total_messages.unwrap().get(), 1);
}

#[test]
fn test_metrics_registration() {
    let registry = Registry::new();

    // Should fail due to duplicate metric names
    let _metrics1 = ChannelMetrics::new("test_dup", "test duplicate metrics", &registry).unwrap();

    let metrics2 = ChannelMetrics::new("test_dup", "test duplicate metrics", &registry);

    assert!(metrics2.is_err());
}
