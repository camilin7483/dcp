//! File system watcher using inotify (Linux) / ReadDirectoryChangesW (Windows) / FSEvents (macOS).
//!
//! Watches specified paths for changes and emits events to the DCP event bus.

use anyhow::Result;
use dcp_types::{EventType, EventData, FileEventData, SystemEvent};
use crate::events::EventBus;
use notify::{Watcher, RecursiveMode, Event, EventKind};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};

/// File watcher that monitors paths and emits DCP events.
pub struct FileWatcher {
    event_bus: EventBus,
    watcher: Arc<tokio::sync::Mutex<Option<notify::RecommendedWatcher>>>,
    watch_tx: mpsc::Sender<Result<Event, notify::Error>>,
    watch_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<Result<Event, notify::Error>>>>,
}

impl FileWatcher {
    pub fn new(event_bus: EventBus) -> Self {
        let (tx, rx) = mpsc::channel(256);
        Self {
            event_bus,
            watcher: Arc::new(tokio::sync::Mutex::new(None)),
            watch_tx: tx,
            watch_rx: Arc::new(tokio::sync::Mutex::new(rx)),
        }
    }

    /// Start watching a path for changes.
    pub async fn watch(&self, path: &Path, recursive: bool) -> Result<()> {
        let tx = self.watch_tx.clone();
        
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            let _ = tx.blocking_send(res);
        })?;

        let mode = if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        watcher.watch(path, mode)?;
        *self.watcher.lock().await = Some(watcher);

        info!("Started watching: {}", path.display());
        Ok(())
    }

    /// Stop watching all paths.
    pub async fn stop(&self) {
        *self.watcher.lock().await = None;
        info!("Stopped file watcher");
    }

    /// Run the event processing loop.
    pub async fn run(&self) -> Result<()> {
        let mut rx = self.watch_rx.lock().await;

        while let Some(res) = rx.recv().await {
            match res {
                Ok(event) => {
                    if let Some(dcp_event) = self.convert_event(event) {
                        self.event_bus.publish(dcp_event).await;
                    }
                }
                Err(e) => {
                    warn!("File watcher error: {e}");
                }
            }
        }

        Ok(())
    }

    fn convert_event(&self, event: Event) -> Option<SystemEvent> {
        let path = event.paths.first()?;
        let path_str = path.to_string_lossy().to_string();

        let event_type = match event.kind {
            EventKind::Create(_) => EventType::FileCreated,
            EventKind::Modify(_) => EventType::FileChanged,
            EventKind::Remove(_) => EventType::FileDeleted,
            _ => return None,
        };

        let data = EventData::File(FileEventData {
            path: path_str,
            old_path: None,
        });

        Some(SystemEvent::new(event_type, data))
    }
}

/// Run the file watcher on a set of default paths.
pub async fn run_file_watcher(event_bus: EventBus, watch_paths: Vec<PathBuf>) -> Result<()> {
    let watcher = FileWatcher::new(event_bus);

    for path in watch_paths {
        if path.exists() {
            if let Err(e) = watcher.watch(&path, true).await {
                warn!("Failed to watch {}: {e}", path.display());
            }
        }
    }

    watcher.run().await
}
