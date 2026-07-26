//! The [`EventBus`] — an in-process, typed pub/sub built on
//! `tokio::sync::broadcast`.
//!
//! Delivery is **best-effort to active subscribers** (see
//! [ADR-0007](../../../docs/adrs/0007-best-effort-event-delivery.md)): the
//! channel is bounded (capacity ≥ 1024 by default), and a slow subscriber that
//! falls behind the buffer window observes a
//! [`Lagged`](tokio::sync::broadcast::error::RecvError::Lagged) condition and
//! must reconcile from canonical state. Events are notifications, not a durable
//! log; handlers must be idempotent (duplicate notifications and rebuilds are
//! valid). The hot path (keystroke → editing transaction → render) never goes
//! through the bus.
//!
//! Shutdown is driven externally via a `CancellationToken` (from `tokio-util`);
//! subscribers combine it with [`EventSubscriber::recv`] using `tokio::select!`:
//!
//! ```no_run
//! use tokio_util::sync::CancellationToken;
//! use itala_kernel::bus::{EventBus, EventSubscriber};
//!
//! # async fn _doc() {
//! let bus = EventBus::new();
//! let mut sub = bus.subscribe();
//! let token = CancellationToken::new();
//! tokio::select! {
//!     _ = token.cancelled() => { /* shutting down */ }
//!     ev = sub.recv() => { /* handle event */ }
//! }
//! # }
//! ```

use tokio::sync::broadcast;
use tokio::sync::broadcast::error::{RecvError, SendError};

use crate::events::KernelEvent;

/// Default bounded capacity. The architecture requires at least 1024.
pub const DEFAULT_CAPACITY: usize = 1024;

/// The typed publish side of the event bus.
///
/// Clone cheaply to share the bus across tasks; every clone publishes into the
/// same channel.
#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<KernelEvent>,
    capacity: usize,
}

impl EventBus {
    /// Creates a new bus with the default bounded capacity ([`DEFAULT_CAPACITY`]).
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// Creates a new bus with a custom bounded capacity.
    ///
    /// `capacity` must be at least 1; smaller values are clamped to 1.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        let (tx, _rx) = broadcast::channel(capacity);
        Self { tx, capacity }
    }

    /// Returns a new subscriber. Each subscriber receives a copy of every
    /// subsequently-published event.
    #[must_use]
    pub fn subscribe(&self) -> EventSubscriber {
        EventSubscriber {
            rx: self.tx.subscribe(),
        }
    }

    /// Publishes an event to all current subscribers.
    ///
    /// Returns `Ok(())` if at least one subscriber received the event, or
    /// `Err` if there are currently no subscribers (which is not a failure
    /// condition for fire-and-forget publishers). Use [`EventBus::publish`]
    /// to ignore the no-receiver case.
    pub fn try_publish(&self, event: KernelEvent) -> Result<usize, SendError<KernelEvent>> {
        self.tx.send(event)
    }

    /// Publishes an event, ignoring the no-subscriber case.
    pub fn publish(&self, event: KernelEvent) {
        let _ = self.tx.send(event);
    }

    /// Returns the configured channel capacity.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the current number of subscribers.
    #[must_use]
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// The subscribe side of the event bus.
pub struct EventSubscriber {
    rx: broadcast::Receiver<KernelEvent>,
}

impl EventSubscriber {
    /// Receives the next event.
    ///
    /// Returns `Err(RecvError::Closed)` when all senders have been dropped and
    /// the buffer is drained, or `Err(RecvError::Lagged(n))` if the subscriber
    /// fell behind the buffer window by `n` messages and must resync.
    ///
    /// # Errors
    /// See [`RecvError`].
    pub async fn recv(&mut self) -> Result<KernelEvent, RecvError> {
        self.rx.recv().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NoteId, PluginId};
    use tokio_util::sync::CancellationToken;

    #[tokio::test]
    async fn two_subscribers_each_receive_all_events() {
        const N: usize = 1024;

        async fn collect(mut sub: EventSubscriber, expected: usize) -> usize {
            let mut got = 0usize;
            while sub.recv().await.unwrap() == KernelEvent::IndexRebuildRequested {
                got += 1;
                if got == expected {
                    break;
                }
            }
            got
        }

        let bus = EventBus::new();
        let a = bus.subscribe();
        let b = bus.subscribe();

        for _ in 0..N {
            bus.publish(KernelEvent::IndexRebuildRequested);
        }

        let (ra, rb) = tokio::join!(collect(a, N), collect(b, N));
        assert_eq!(ra, N);
        assert_eq!(rb, N);
    }

    #[tokio::test]
    async fn cancellation_exits_select_cleanly() {
        let bus = EventBus::new();
        let mut sub = bus.subscribe();
        let token = CancellationToken::new();
        let cancel = token.clone();

        let handle = tokio::spawn(async move {
            tokio::select! {
                biased;
                () = cancel.cancelled() => "cancelled",
                _ = sub.recv() => "event",
            }
        });

        token.cancel();
        assert_eq!(handle.await.unwrap(), "cancelled");
    }

    #[tokio::test]
    async fn closed_returns_none_after_drain() {
        let bus = EventBus::new();
        let mut sub = bus.subscribe();
        bus.publish(KernelEvent::NoteSaved { id: NoteId::new() });
        drop(bus);
        // first recv drains the buffered event
        assert!(sub.recv().await.is_ok());
        // then the channel is closed
        match sub.recv().await {
            Err(RecvError::Closed) => {}
            other => panic!("expected Closed, got {other:?}"),
        }
    }

    #[test]
    fn capacity_is_at_least_1024_by_default() {
        let bus = EventBus::new();
        assert!(bus.capacity() >= DEFAULT_CAPACITY);
    }

    #[test]
    fn subscriber_count_tracks_subscriptions() {
        let bus = EventBus::new();
        assert_eq!(bus.subscriber_count(), 0);
        let _a = bus.subscribe();
        let _b = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 2);
    }

    #[test]
    fn carries_typed_payloads() {
        let bus = EventBus::new();
        let mut sub = bus.subscribe();
        let id = NoteId::new();
        let pid = PluginId::new("com.example.p");
        bus.publish(KernelEvent::NoteSaved { id });
        bus.publish(KernelEvent::PluginLoaded { id: pid });
        // block-on via current-thread
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            assert_eq!(sub.recv().await.unwrap(), KernelEvent::NoteSaved { id });
            assert!(matches!(
                sub.recv().await.unwrap(),
                KernelEvent::PluginLoaded { .. }
            ));
        });
    }
}
