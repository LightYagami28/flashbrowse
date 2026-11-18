//! Async CDP event dispatcher.
//!
//! The transport layer routes any frame containing `method` (and no `id`) to
//! [`EventEmitter::emit`]. Callers subscribe with [`EventEmitter::subscribe`]
//! to obtain a [`broadcast::Receiver`] for a given method name, e.g.
//! `Page.loadEventFired`.

use std::sync::atomic::{AtomicU64, Ordering};

use dashmap::DashMap;
use serde_json::Value;
use tokio::sync::broadcast;

/// Broadcast capacity per method. Events are fire-and-forget; slow receivers
/// that fall behind see `Lagged` and should re-sync from fresh state.
const CHANNEL_CAPACITY: usize = 64;

/// Routes CDP events to subscribers by method name.
///
/// `EventEmitter` is cheap to clone via `Arc`. Counters are lock-free
/// atomics so `emit` never stalls on a full lock under load (M13).
pub struct EventEmitter {
    subscribers: DashMap<String, Vec<broadcast::Sender<Value>>>,
    counts: DashMap<String, AtomicU64>,
}

impl EventEmitter {
    /// Create an empty emitter.
    pub fn new() -> Self {
        Self {
            subscribers: DashMap::new(),
            counts: DashMap::new(),
        }
    }

    /// Subscribe to a CDP method, e.g. `"Page.loadEventFired"`.
    ///
    /// Multiple subscribers to the same method each get their own channel.
    pub fn subscribe(&self, method: &str) -> broadcast::Receiver<Value> {
        let (sender, receiver) = broadcast::channel(CHANNEL_CAPACITY);
        // Prune dead senders on subscribe too (not only on emit) so a
        // subscribe-heavy workload with no events cannot grow unboundedly.
        self.subscribers
            .entry(method.to_string())
            .or_default()
            .retain(|s| s.receiver_count() > 0);
        if let Some(mut senders) = self.subscribers.get_mut(method) {
            senders.push(sender);
        } else {
            self.subscribers.insert(method.to_string(), vec![sender]);
        }
        receiver
    }

    /// Emit an event `params` payload to all subscribers of `method`.
    ///
    /// Dead senders (dropped receivers) are removed lazily.
    pub fn emit(&self, method: &str, params: Value) {
        if let Some(mut senders) = self.subscribers.get_mut(method) {
            senders.retain(|s| s.receiver_count() > 0);
            for sender in senders.iter() {
                let _ = sender.send(params.clone());
            }
            if senders.is_empty() {
                drop(senders);
                self.subscribers.remove_if(method, |_, v| v.is_empty());
            }
        }
        self.counts
            .entry(method.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Approximate number of events seen per method (for metrics/debug).
    pub fn event_count(&self, method: &str) -> u64 {
        self.counts
            .get(method)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }
}

impl Default for EventEmitter {
    fn default() -> Self {
        Self::new()
    }
}
