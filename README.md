# Metered Bounded Channel

The metered bounded channel is a specialized threading utility designed to handle communication between threads with an upper limit on capacity while tracking the channel's occupancy through Prometheus metrics.

## Functionality
- **Bounded Capacity**: This channel ensures that no more than a predefined number of messages are held in the channel at any given time.
- **Backpressure Handling**: When the channel reaches its capacity, any additional attempts to send messages will be blocked, allowing for backpressure management until the channel has available space.
- **Prometheus Integration**: The current occupancy of the channel is exposed as a Prometheus metric, enabling real-time monitoring of how "full" the channel is.