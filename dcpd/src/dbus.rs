//! D-Bus notification listener for Linux.
//!
//! Subscribes to org.freedesktop.Notifications on the session bus
//! and forwards notification events into the DCP event bus.

use crate::events::EventBus;
use anyhow::Result;
use dcp_types::{EventData, EventType, NotificationEventData, SystemEvent};
use tracing::{info, warn};

/// Listens for D-Bus notification signals and publishes them to the event bus.
pub async fn run_notification_listener(event_bus: EventBus) -> Result<()> {
    info!("Starting D-Bus notification listener");

    let connection = zbus::Connection::session().await?;
    let mut stream = zbus::MessageStream::from(&connection);

    info!("Listening for notifications on org.freedesktop.Notifications");

    use futures::StreamExt;
    while let Some(msg) = stream.next().await {
        let msg = match msg {
            Ok(m) => m,
            Err(e) => {
                warn!("D-Bus message error: {e}");
                continue;
            }
        };

        let header = msg.header();
        let member = match header.member() {
            Some(m) => m,
            None => continue,
        };

        if member.as_str() != "Notify" {
            continue;
        }

        let body = msg.body();

        let data = match body.deserialize::<(
            String,                                                   // app_name
            u32,                                                      // replaces_id
            String,                                                   // app_icon
            String,                                                   // summary
            String,                                                   // body
            Vec<String>,                                              // actions
            std::collections::HashMap<String, zbus::zvariant::Value>, // hints
            i32,                                                      // expire_timeout
        )>() {
            Ok((
                app_name,
                _replaces_id,
                _app_icon,
                summary,
                notification_body,
                _actions,
                _hints,
                _expire_timeout,
            )) => EventData::Notification(NotificationEventData {
                id: uuid::Uuid::new_v4().to_string(),
                app_name,
                title: summary,
                body: Some(notification_body),
                action: None,
            }),
            Err(e) => {
                warn!("Failed to parse notification: {e}");
                continue;
            }
        };

        let event = SystemEvent::new(EventType::NotificationReceived, data);
        event_bus.publish(event).await;
    }

    Ok(())
}
