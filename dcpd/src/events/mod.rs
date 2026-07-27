//! Event bus: fan-out, subscription management, batching, coalescing.
//!
//! Supports two delivery modes:
//! - **Immediate**: events delivered as they arrive
//! - **Batched**: events coalesced within a time window before delivery

use dcp_types::{EventType, SystemEvent};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, broadcast, mpsc, watch};
use tokio::time;
use tracing::warn;

/// Central event bus for broadcasting system events to subscribers.
#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<SystemEvent>,
    subscriptions: Arc<RwLock<HashMap<String, SubscriptionState>>>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(8192);
        Self {
            sender,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn publish(&self, event: SystemEvent) {
        let _ = self.sender.send(event);
    }

    pub async fn publish_many(&self, events: Vec<SystemEvent>) {
        for event in events {
            let _ = self.sender.send(event);
        }
    }

    /// Subscribe to specific event types. Returns a subscription ID
    /// and an mpsc receiver for batched event delivery.
    pub async fn subscribe(
        &self,
        event_types: Vec<EventType>,
        batch: bool,
        batch_interval_ms: Option<u64>,
    ) -> (String, mpsc::Receiver<Vec<SystemEvent>>) {
        let sub_id = format!("sub_{}", uuid::Uuid::new_v4());
        let receiver = self.sender.subscribe();

        let interval = if batch {
            Duration::from_millis(batch_interval_ms.unwrap_or(100))
        } else {
            Duration::from_millis(0)
        };

        let (tx, rx) = mpsc::channel(256);
        let type_set: HashSet<EventType> = event_types.into_iter().collect();

        let (cancel_tx, cancel_rx) = watch::channel(false);

        let state = SubscriptionState {
            id: sub_id.clone(),
            event_types: type_set,
            batch: interval > Duration::ZERO,
            batch_interval: interval,
            cancel_tx: Some(cancel_tx),
        };

        self.subscriptions
            .write()
            .await
            .insert(sub_id.clone(), state);

        // Spawn the event delivery task
        let subs = self.subscriptions.clone();
        let delivery_id = sub_id.clone();
        tokio::spawn(async move {
            Self::run_delivery(delivery_id, receiver, tx, subs, cancel_rx).await;
        });

        (sub_id, rx)
    }

    pub async fn unsubscribe(&self, sub_id: &str) -> bool {
        self.subscriptions.write().await.remove(sub_id).is_some()
    }

    pub async fn cancel_subscription(&self, sub_id: &str) -> bool {
        let subs = self.subscriptions.read().await;
        if let Some(state) = subs.get(sub_id) {
            if let Some(cancel_tx) = &state.cancel_tx {
                let _ = cancel_tx.send(true);
                return true;
            }
        }
        false
    }

    pub async fn active_subscriptions(&self) -> Vec<String> {
        self.subscriptions.read().await.keys().cloned().collect()
    }

    async fn run_delivery(
        sub_id: String,
        mut receiver: broadcast::Receiver<SystemEvent>,
        tx: mpsc::Sender<Vec<SystemEvent>>,
        subs: Arc<RwLock<HashMap<String, SubscriptionState>>>,
        mut cancel_rx: watch::Receiver<bool>,
    ) {
        let mut batch_buffer: Vec<SystemEvent> = Vec::new();
        let batch_deadline = time::sleep(Duration::from_secs(3600));
        tokio::pin!(batch_deadline);

        loop {
            let should_batch = subs
                .read()
                .await
                .get(&sub_id)
                .map(|s| s.batch)
                .unwrap_or(false);

            let type_filter = subs
                .read()
                .await
                .get(&sub_id)
                .map(|s| s.event_types.clone())
                .unwrap_or_default();

            let interval = subs
                .read()
                .await
                .get(&sub_id)
                .map(|s| s.batch_interval)
                .unwrap_or(Duration::ZERO);

            if should_batch && !batch_buffer.is_empty() {
                batch_deadline
                    .as_mut()
                    .reset(time::Instant::now() + interval);
            }

            tokio::select! {
                event = receiver.recv() => {
                    match event {
                        Ok(sys_event) => {
                            // Filter by subscribed types
                            if !type_filter.is_empty() && !type_filter.contains(&sys_event.event_type) {
                                continue;
                            }

                            if should_batch {
                                // Coalesce: if same event type already in buffer, replace it
                                let dominated = batch_buffer.iter_mut().any(|existing| {
                                    if existing.event_type == sys_event.event_type {
                                        *existing = sys_event.clone();
                                        true
                                    } else {
                                        false
                                    }
                                });
                                if !dominated {
                                    batch_buffer.push(sys_event);
                                }
                            } else {
                                // Immediate delivery
                                if tx.send(vec![sys_event]).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            warn!("Subscription {sub_id} lagged, dropped {n} events");
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            break;
                        }
                    }
                }
                _ = &mut batch_deadline, if should_batch && !batch_buffer.is_empty() => {
                    if !batch_buffer.is_empty() {
                        let batch = std::mem::take(&mut batch_buffer);
                        if tx.send(batch).await.is_err() {
                            break;
                        }
                    }
                }
                _ = cancel_rx.changed() => {
                    // Cancel signal received, flush and exit
                    break;
                }
            }
        }

        // Flush remaining
        if !batch_buffer.is_empty() {
            let _ = tx.send(batch_buffer).await;
        }

        subs.write().await.remove(&sub_id);
    }
}

struct SubscriptionState {
    id: String,
    event_types: HashSet<EventType>,
    batch: bool,
    batch_interval: Duration,
    cancel_tx: Option<watch::Sender<bool>>,
}
