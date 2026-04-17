use notify::{Config as NotifyConfig, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};
use tao::event_loop::EventLoopProxy;

use crate::app::AppEvent;

pub struct FileWatcher {
    _watcher: RecommendedWatcher,
    rx: Receiver<Result<Event, notify::Error>>,
    last_fire: Instant,
    debounce_ms: u64,
    watched_file: PathBuf,
}

impl FileWatcher {
    pub fn new(file_path: &Path, proxy: EventLoopProxy<AppEvent>) -> Result<Self, String> {
        let (tx, rx): (Sender<Result<Event, notify::Error>>, Receiver<_>) = channel();

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                let _ = tx.send(res);
            },
            NotifyConfig::default().with_poll_interval(Duration::from_millis(500)),
        )
        .map_err(|e| e.to_string())?;

        let parent = file_path
            .parent()
            .ok_or("Cannot determine parent directory")?;

        watcher
            .watch(parent, RecursiveMode::NonRecursive)
            .map_err(|e| e.to_string())?;

        let watched_file = file_path.to_path_buf();

        // Spawn a background thread to poll for file changes
        // and send AppEvent::FileChanged via the event loop proxy
        let proxy_clone = proxy;
        let _watched = watched_file.clone();
        let _rx_poll = rx.try_recv().ok(); // drain initial events

        // We need a second channel for the polling thread since Receiver is not Clone
        let (tx_poll, rx_poll): (Sender<Result<Event, notify::Error>>, Receiver<_>) = channel();

        // Create a second watcher for the polling thread
        let mut poll_watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                let _ = tx_poll.send(res);
            },
            NotifyConfig::default().with_poll_interval(Duration::from_millis(500)),
        )
        .map_err(|e| e.to_string())?;

        poll_watcher
            .watch(parent, RecursiveMode::NonRecursive)
            .map_err(|e| e.to_string())?;

        let file_name = watched_file
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        std::thread::spawn(move || {
            let mut last_fire = Instant::now();
            let debounce_ms: u128 = 200;

            loop {
                std::thread::sleep(Duration::from_millis(200));

                let mut should_fire = false;
                while let Ok(result) = rx_poll.try_recv() {
                    match result {
                        Ok(event) => {
                            if matches!(
                                event.kind,
                                EventKind::Modify(_) | EventKind::Create(_) | EventKind::Any
                            ) {
                                let matches = event.paths.iter().any(|p: &PathBuf| {
                                    p.file_name()
                                        .map(|n| n.to_string_lossy() == file_name)
                                        .unwrap_or(false)
                                });
                                if matches {
                                    should_fire = true;
                                }
                            }
                        }
                        Err(_) => continue,
                    }
                }

                if should_fire {
                    let now = Instant::now();
                    if now.duration_since(last_fire).as_millis() > debounce_ms {
                        last_fire = now;
                        let _ = proxy_clone.send_event(AppEvent::FileChanged);
                    }
                }
            }
        });

        Ok(Self {
            _watcher: watcher,
            rx,
            last_fire: Instant::now(),
            debounce_ms: 200,
            watched_file,
        })
    }

    #[allow(dead_code)]
    pub fn check_changed(&mut self) -> bool {
        let file_name = self.watched_file.file_name();

        while let Ok(result) = self.rx.try_recv() {
            match result {
                Ok(event) => {
                    if matches!(
                        event.kind,
                        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Any
                    ) {
                        let matches = event.paths.iter().any(|p: &PathBuf| {
                            p.file_name() == file_name
                                && p.parent() == self.watched_file.parent()
                        });
                        if matches {
                            let now = Instant::now();
                            if now.duration_since(self.last_fire).as_millis()
                                > self.debounce_ms as u128
                            {
                                self.last_fire = now;
                                return true;
                            }
                        }
                    }
                }
                Err(_) => continue,
            }
        }
        false
    }
}
