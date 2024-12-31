use prometheus::{IntCounter, IntGauge, Opts, Registry};

/// Metrics for channel monitoring
#[derive(Clone, Debug)]
pub struct ChannelMetrics {
    /// Current number of items in the channel
    pub queue_size: IntGauge,
    /// Total number of items that have gone through the channel
    pub total_messages: Option<IntCounter>,
}

impl ChannelMetrics {
    /// Create new channel metrics and register them with Prometheus
    pub fn new(name: &str, help: &str, registry: &Registry) -> Result<Self, prometheus::Error> {
        let queue_size = IntGauge::with_opts(Opts::new(
            format!("{}_queue_size", name),
            format!("Current number of items in {} channel", help),
        ))?;
        registry.register(Box::new(queue_size.clone()))?;

        let total_messages = IntCounter::with_opts(Opts::new(
            format!("{}_total_messages", name),
            format!("Total number of messages processed by {} channel", help),
        ))?;
        registry.register(Box::new(total_messages.clone()))?;

        Ok(Self {
            queue_size,
            total_messages: Some(total_messages),
        })
    }

    /// Create metrics without total message counter
    pub fn new_basic(
        name: &str,
        help: &str,
        registry: &Registry,
    ) -> Result<Self, prometheus::Error> {
        let queue_size = IntGauge::with_opts(Opts::new(
            format!("{}_queue_size", name),
            format!("Current number of items in {} channel", help),
        ))?;
        registry.register(Box::new(queue_size.clone()))?;

        Ok(Self {
            queue_size,
            total_messages: None,
        })
    }
}
