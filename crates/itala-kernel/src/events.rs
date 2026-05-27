//! Kernel-level event bus.
//!
//! [`EventBus<E>`] is a generic broadcast primitive over any
//! `Clone + Send + 'static` payload, backed by a bounded
//! [`tokio::sync::broadcast`] channel (capacity 1024). Publishing is
//! fire-and-forget: if no subscribers are listening, the event is silently
//! dropped. Subscribers that fall too far behind receive
//! [`tokio::sync::broadcast::error::RecvError::Lagged`].
//!
//! The kernel ships one concrete event type today, [`KernelEvent`], with a
//! convenience alias [`KernelEventBus`]. Bounded-context crates introduced
//! later (vault, note-editor, graph, …) will define their own event enums and
//! parameterize [`EventBus`] over them.
//!
//! # Usage
//!
//! ```
//! use itala_kernel::{EventBus, KernelEvent, NoteId, VaultId};
//!
//! let bus: EventBus<KernelEvent> = EventBus::new();
//! let mut rx = bus.subscribe();
//!
//! let vault_id = VaultId::new();
//! let note_id  = NoteId::new();
//! bus.publish(KernelEvent::NoteSaved { vault_id, note_id });
//! ```

use tokio::sync::broadcast;

use crate::{NoteId, VaultId};

/// Capacity of the broadcast ring buffer.
const BUS_CAPACITY: usize = 1024;

/// A semantic domain event emitted by the kernel or vault layer.
///
/// Consumers subscribe via [`EventBus::subscribe`] and pattern-match on
/// variants to react to lifecycle changes.
///
/// The enum is `#[non_exhaustive]` — new variants may be added in minor
/// releases without breaking existing match arms.
//
// TODO(F4): when `itala-vault` is created, this enum splits into
// `vault::VaultEvent` (Opened/Closed/…) and `note::NoteEvent` (Saved/Deleted/…)
// and moves out of the kernel. The [`KernelEventBus`] alias below is the only
// kernel-side reference today; deleting both is the migration.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum KernelEvent {
    /// A vault was successfully opened.
    VaultOpened {
        /// The ID of the vault that was opened.
        vault_id: VaultId,
    },
    /// A vault was closed.
    VaultClosed {
        /// The ID of the vault that was closed.
        vault_id: VaultId,
    },
    /// A note was written (created or updated).
    NoteSaved {
        /// The vault the note belongs to.
        vault_id: VaultId,
        /// The note that was saved.
        note_id: NoteId,
    },
    /// A note was deleted from the vault.
    NoteDeleted {
        /// The vault the note belonged to.
        vault_id: VaultId,
        /// The note that was deleted.
        note_id: NoteId,
    },
}

/// Broadcasts events of type `E` to any number of concurrent subscribers.
///
/// Wraps [`tokio::sync::broadcast`] at a fixed capacity of 1024. Multiple
/// independent parts of the system (vault, search, graph, UI) subscribe and
/// react without coupling to one another.
///
/// `E` is the payload type; it must be `Clone` (broadcast fans out by cloning)
/// and `Send + 'static` (the channel is shared across tasks). Each bounded
/// context crate parameterizes the bus over its own event enum.
///
/// # Cloning and sharing
///
/// `EventBus<E>` is cheap to clone — both the original and the clone share the
/// same underlying channel. Use `Arc<EventBus<E>>` when you need shared
/// ownership across tasks.
///
/// # Back-pressure
///
/// This bus is fire-and-forget. If a slow subscriber's receive buffer overflows,
/// it receives [`tokio::sync::broadcast::error::RecvError::Lagged`] on the next
/// call. Design consumers so they can tolerate missed events or drain promptly.
pub struct EventBus<E: Clone + Send + 'static> {
    sender: broadcast::Sender<E>,
}

impl<E: Clone + Send + 'static> Clone for EventBus<E> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl<E: Clone + Send + 'static> EventBus<E> {
    /// Create a new bus with the default ring-buffer capacity (1024).
    ///
    /// ```
    /// use itala_kernel::{EventBus, KernelEvent};
    /// let bus: EventBus<KernelEvent> = EventBus::new();
    /// assert_eq!(bus.subscriber_count(), 0);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(BUS_CAPACITY);
        Self { sender }
    }

    /// Subscribe to the bus.
    ///
    /// The returned receiver will see all events published *after* this call.
    /// Events published before [`EventBus::subscribe`] is called are not delivered.
    ///
    /// ```
    /// use itala_kernel::{EventBus, KernelEvent};
    /// let bus: EventBus<KernelEvent> = EventBus::new();
    /// let _rx = bus.subscribe();
    /// assert_eq!(bus.subscriber_count(), 1);
    /// ```
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<E> {
        self.sender.subscribe()
    }

    /// Publish an event to all current subscribers.
    ///
    /// If no subscribers are listening the event is silently dropped — a
    /// zero-subscriber send is not an error. If the ring buffer is full,
    /// the oldest event is overwritten (subscribers may see
    /// [`tokio::sync::broadcast::error::RecvError::Lagged`]).
    ///
    /// ```
    /// use itala_kernel::{EventBus, KernelEvent, VaultId};
    /// let bus: EventBus<KernelEvent> = EventBus::new();
    /// // Zero subscribers: publish is a silent no-op, not an error.
    /// bus.publish(KernelEvent::VaultOpened { vault_id: VaultId::new() });
    /// ```
    pub fn publish(&self, event: E) {
        // Ignore SendError — zero subscribers is normal at startup.
        let _ = self.sender.send(event);
    }

    /// Return the number of active subscribers.
    ///
    /// ```
    /// use itala_kernel::{EventBus, KernelEvent};
    /// let bus: EventBus<KernelEvent> = EventBus::new();
    /// let rx1 = bus.subscribe();
    /// let rx2 = bus.subscribe();
    /// assert_eq!(bus.subscriber_count(), 2);
    /// drop(rx1);
    /// drop(rx2);
    /// ```
    #[must_use]
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl<E: Clone + Send + 'static> Default for EventBus<E> {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience alias for the kernel-internal event stream.
///
/// TODO(F4): `KernelEvent` migrates to `itala-vault` (becoming `VaultEvent` +
/// `NoteEvent`); this alias is then deleted, and the kernel exposes only the
/// generic [`EventBus<E>`] primitive.
pub type KernelEventBus = EventBus<KernelEvent>;

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use tokio::sync::Barrier;
    use tokio::task::JoinSet;
    use tokio::time::timeout;
    use tokio_util::sync::CancellationToken;

    use super::*;
    use crate::{NoteId, VaultId};

    fn assert_send_sync<T: Send + Sync>() {}
    fn assert_unpin<T: Unpin>() {}

    // -------------------------------------------------------------------------
    // Compile-time auto-trait assertions
    // -------------------------------------------------------------------------

    #[test]
    fn kernel_event_is_send_sync_unpin() {
        assert_send_sync::<KernelEvent>();
        assert_unpin::<KernelEvent>();
    }

    #[test]
    fn event_bus_is_send_sync() {
        assert_send_sync::<EventBus<KernelEvent>>();
        assert_send_sync::<KernelEventBus>();
    }

    // -------------------------------------------------------------------------
    // Basic publish / subscribe
    // -------------------------------------------------------------------------

    #[test]
    fn publish_with_no_subscribers_does_not_panic() {
        let bus = KernelEventBus::new();
        // Drop the implicit receiver immediately — subscriber_count == 0.
        let vault_id = VaultId::new();
        bus.publish(KernelEvent::VaultOpened { vault_id });
        // No panic → test passes.
    }

    #[tokio::test]
    async fn subscribe_then_publish_delivers_event() {
        let bus = KernelEventBus::new();
        let mut rx = bus.subscribe();

        let vault_id = VaultId::new();
        let note_id = NoteId::new();
        bus.publish(KernelEvent::NoteSaved { vault_id, note_id });

        let event = timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("timed out")
            .expect("recv error");

        assert!(matches!(event, KernelEvent::NoteSaved { .. }));
    }

    #[tokio::test]
    async fn default_constructs_a_working_bus() {
        let bus = KernelEventBus::default();
        let mut rx = bus.subscribe();
        bus.publish(KernelEvent::VaultClosed {
            vault_id: VaultId::new(),
        });
        let event = rx.recv().await.unwrap();
        assert!(matches!(event, KernelEvent::VaultClosed { .. }));
    }

    #[tokio::test]
    async fn clone_shares_channel() {
        let bus = KernelEventBus::new();
        let bus2 = bus.clone();
        let mut rx = bus.subscribe();

        bus2.publish(KernelEvent::VaultOpened {
            vault_id: VaultId::new(),
        });

        let event = timeout(Duration::from_secs(1), rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(event, KernelEvent::VaultOpened { .. }));
    }

    // -------------------------------------------------------------------------
    // Generic-instantiation: bus works for any Clone + Send + 'static payload
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn bus_is_generic_over_payload() {
        #[derive(Clone, Debug, PartialEq)]
        struct Pulse(u32);

        let bus: EventBus<Pulse> = EventBus::new();
        let mut rx = bus.subscribe();
        bus.publish(Pulse(42));
        let pulse = timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("timed out")
            .expect("recv error");
        assert_eq!(pulse, Pulse(42));
    }

    // -------------------------------------------------------------------------
    // Step 5: Multi-subscriber integration test (1000 events, 2 receivers)
    // -------------------------------------------------------------------------

    #[tokio::test(flavor = "multi_thread")]
    async fn two_subscribers_receive_all_1k_events() {
        const N: usize = 1000;

        let bus = Arc::new(KernelEventBus::new());
        // Create both subscriptions BEFORE publishing — use a Barrier to ensure
        // both receiver tasks are waiting before the publisher fires any events.
        let barrier = Arc::new(Barrier::new(3)); // 2 subscribers + 1 publisher

        let mut tasks = JoinSet::new();
        for _ in 0..2 {
            let mut rx = bus.subscribe();
            let barrier = Arc::clone(&barrier);
            tasks.spawn(async move {
                barrier.wait().await;
                let mut received = Vec::with_capacity(N);
                for _ in 0..N {
                    let event = rx.recv().await.expect("event bus closed early");
                    received.push(event);
                }
                received.len()
            });
        }

        // Publisher waits for both subscribers to be ready.
        barrier.wait().await;
        for _ in 0..N {
            bus.publish(KernelEvent::NoteSaved {
                vault_id: VaultId::new(),
                note_id: NoteId::new(),
            });
        }

        while let Some(result) = tasks.join_next().await {
            let count = result.expect("subscriber task panicked");
            assert_eq!(count, N, "subscriber missed events");
        }
    }

    // -------------------------------------------------------------------------
    // Step 5: Cancellation test
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn cancellation_exits_select_loop() {
        let bus = KernelEventBus::new();
        let mut rx = bus.subscribe();
        let token = CancellationToken::new();
        let token_clone = token.clone();

        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    () = token_clone.cancelled() => break,
                    result = rx.recv() => {
                        // Process event (or ignore lag).
                        let _ = result;
                    }
                }
            }
        });

        // Give the task a moment to enter the select, then cancel.
        token.cancel();

        timeout(Duration::from_secs(1), task)
            .await
            .expect("loop did not exit within 1s")
            .expect("task panicked");
    }
}
