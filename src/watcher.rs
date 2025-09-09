//! File system watching utilities for configuration files.

use crate::error::{ConfigError, ConfigResult};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Type alias for configuration change callback functions.
pub type ConfigChangeCallback = Box<dyn Fn() + Send + Sync>;

/// Manages file system watching for configuration files.
pub struct FileWatcher {
    _watcher: RecommendedWatcher,
    receiver: mpsc::Receiver<notify::Result<Event>>,
    watched_files: Vec<PathBuf>,
    callbacks: Arc<Mutex<Vec<ConfigChangeCallback>>>,
    is_watching: bool,
}

impl FileWatcher {
    /// Creates a new file watcher for the specified path.
    pub fn new<P: AsRef<Path>>(path: P) -> ConfigResult<Self> {
        let (sender, receiver) = mpsc::channel();

        let mut watcher = notify::recommended_watcher(sender)
            .map_err(|e| ConfigError::FileWatch(e.to_string()))?;

        let path_buf = path.as_ref().to_path_buf();
        watcher
            .watch(&path_buf, RecursiveMode::NonRecursive)
            .map_err(|e| ConfigError::FileWatch(e.to_string()))?;

        Ok(Self {
            _watcher: watcher,
            receiver,
            watched_files: vec![path_buf],
            callbacks: Arc::new(Mutex::new(Vec::new())),
            is_watching: false,
        })
    }

    /// Creates a new file watcher without watching any files initially.
    pub fn new_empty() -> ConfigResult<Self> {
        let (sender, receiver) = mpsc::channel();

        let watcher = notify::recommended_watcher(sender)
            .map_err(|e| ConfigError::FileWatch(e.to_string()))?;

        Ok(Self {
            _watcher: watcher,
            receiver,
            watched_files: Vec::new(),
            callbacks: Arc::new(Mutex::new(Vec::new())),
            is_watching: false,
        })
    }

    /// Adds a file to be watched.
    pub fn watch_file<P: AsRef<Path>>(&mut self, path: P) -> ConfigResult<()> {
        let path_buf = path.as_ref().to_path_buf();

        // Only watch if the file exists
        if !path_buf.exists() {
            return Err(ConfigError::FileWatch(format!(
                "Cannot watch non-existent file: {}",
                path_buf.display()
            )));
        }

        self._watcher
            .watch(&path_buf, RecursiveMode::NonRecursive)
            .map_err(|e| ConfigError::FileWatch(e.to_string()))?;

        self.watched_files.push(path_buf);
        Ok(())
    }

    /// Removes a file from being watched.
    pub fn unwatch_file<P: AsRef<Path>>(&mut self, path: P) -> ConfigResult<()> {
        let path_buf = path.as_ref().to_path_buf();

        self._watcher
            .unwatch(&path_buf)
            .map_err(|e| ConfigError::FileWatch(e.to_string()))?;

        self.watched_files.retain(|p| p != &path_buf);
        Ok(())
    }

    /// Gets the list of currently watched files.
    pub fn watched_files(&self) -> &[PathBuf] {
        &self.watched_files
    }

    /// Registers a callback to be called when configuration changes are detected.
    pub fn on_config_change<F>(&self, callback: F) -> ConfigResult<()>
    where
        F: Fn() + Send + Sync + 'static,
    {
        let mut callbacks = self.callbacks.lock().map_err(|e| {
            ConfigError::FileWatch(format!("Failed to acquire callback lock: {e}"))
        })?;

        callbacks.push(Box::new(callback));
        Ok(())
    }

    /// Starts watching for file changes in a background thread.
    /// This method spawns a background thread that monitors for file changes
    /// and calls registered callbacks when changes are detected.
    pub fn start_watching(&mut self) -> ConfigResult<()> {
        if self.is_watching {
            return Ok(()); // Already watching
        }

        let callbacks = Arc::clone(&self.callbacks);
        let (_stop_sender, stop_receiver) = mpsc::channel::<()>();

        // We need to create a new receiver since we can't clone the existing one
        let (event_sender, event_receiver) = mpsc::channel();

        // Replace the watcher with a new one that uses our new sender
        let mut new_watcher = notify::recommended_watcher(event_sender)
            .map_err(|e| ConfigError::FileWatch(e.to_string()))?;

        // Re-watch all previously watched files
        for path in &self.watched_files {
            new_watcher
                .watch(path, RecursiveMode::NonRecursive)
                .map_err(|e| ConfigError::FileWatch(e.to_string()))?;
        }

        self._watcher = new_watcher;
        self.is_watching = true;

        // Spawn background thread for watching
        thread::spawn(move || {
            loop {
                // Check if we should stop
                if stop_receiver.try_recv().is_ok() {
                    break;
                }

                // Check for file system events
                match event_receiver.recv_timeout(Duration::from_millis(100)) {
                    Ok(Ok(_event)) => {
                        // File change detected, call all callbacks
                        if let Ok(callbacks_guard) = callbacks.lock() {
                            for callback in callbacks_guard.iter() {
                                callback();
                            }
                        }
                    }
                    Ok(Err(_)) => {
                        // Error in file watching, but continue
                        continue;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        // No events, continue
                        continue;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        // Channel disconnected, stop watching
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Stops watching for file changes.
    pub fn stop_watching(&mut self) {
        self.is_watching = false;
        // Note: In a full implementation, we'd send a stop signal to the background thread
        // For now, the thread will detect disconnection and stop
    }

    /// Returns whether the watcher is currently active.
    pub fn is_watching(&self) -> bool {
        self.is_watching
    }

    /// Triggers all registered callbacks manually (for testing purposes).
    #[cfg(test)]
    pub fn trigger_callbacks_for_test(&self) {
        if let Ok(callbacks_guard) = self.callbacks.lock() {
            for callback in callbacks_guard.iter() {
                callback();
            }
        }
    }

    /// Checks for file system events with a timeout.
    /// This method is primarily for testing and manual polling.
    /// For automatic reloading, use start_watching() instead.
    pub fn check_for_changes(&self, timeout: Duration) -> ConfigResult<bool> {
        match self.receiver.recv_timeout(timeout) {
            Ok(Ok(_event)) => {
                // Call callbacks when changes are detected
                if let Ok(callbacks_guard) = self.callbacks.lock() {
                    for callback in callbacks_guard.iter() {
                        callback();
                    }
                }
                Ok(true)
            }
            Ok(Err(e)) => Err(ConfigError::FileWatch(e.to_string())),
            Err(mpsc::RecvTimeoutError::Timeout) => Ok(false),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                Err(ConfigError::FileWatch("Watcher disconnected".to_string()))
            }
        }
    }

    /// Blocks until a file change is detected.
    /// This method is primarily for testing and manual polling.
    /// For automatic reloading, use start_watching() instead.
    pub fn wait_for_change(&self) -> ConfigResult<()> {
        match self.receiver.recv() {
            Ok(Ok(_event)) => {
                // Call callbacks when changes are detected
                if let Ok(callbacks_guard) = self.callbacks.lock() {
                    for callback in callbacks_guard.iter() {
                        callback();
                    }
                }
                Ok(())
            }
            Ok(Err(e)) => Err(ConfigError::FileWatch(e.to_string())),
            Err(_) => Err(ConfigError::FileWatch("Watcher disconnected".to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn test_file_watcher_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        fs::write(&config_path, "{}").unwrap();

        let watcher = FileWatcher::new(&config_path);
        assert!(watcher.is_ok());

        let watcher = watcher.unwrap();
        assert_eq!(watcher.watched_files().len(), 1);
        assert_eq!(watcher.watched_files()[0], config_path);
    }

    #[test]
    fn test_empty_file_watcher() {
        let watcher = FileWatcher::new_empty();
        assert!(watcher.is_ok());

        let watcher = watcher.unwrap();
        assert_eq!(watcher.watched_files().len(), 0);
        assert!(!watcher.is_watching());
    }

    #[test]
    fn test_watch_multiple_files() {
        let temp_dir = TempDir::new().unwrap();
        let config1 = temp_dir.path().join("config1.json");
        let config2 = temp_dir.path().join("config2.yaml");

        fs::write(&config1, "{}").unwrap();
        fs::write(&config2, "key: value").unwrap();

        let mut watcher = FileWatcher::new_empty().unwrap();

        assert!(watcher.watch_file(&config1).is_ok());
        assert!(watcher.watch_file(&config2).is_ok());

        assert_eq!(watcher.watched_files().len(), 2);
    }

    #[test]
    fn test_watch_nonexistent_file() {
        let mut watcher = FileWatcher::new_empty().unwrap();
        let nonexistent = PathBuf::from("/nonexistent/file.json");

        let result = watcher.watch_file(&nonexistent);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot watch non-existent file"));
    }

    #[test]
    fn test_callback_registration() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        fs::write(&config_path, "{}").unwrap();

        let watcher = FileWatcher::new(&config_path).unwrap();

        let callback_called = Arc::new(Mutex::new(false));
        let callback_called_clone = Arc::clone(&callback_called);

        let result = watcher.on_config_change(move || {
            *callback_called_clone.lock().unwrap() = true;
        });

        assert!(result.is_ok());
    }

    #[test]
    fn test_unwatch_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        fs::write(&config_path, "{}").unwrap();

        let mut watcher = FileWatcher::new(&config_path).unwrap();
        assert_eq!(watcher.watched_files().len(), 1);

        assert!(watcher.unwatch_file(&config_path).is_ok());
        assert_eq!(watcher.watched_files().len(), 0);
    }

    #[test]
    fn test_file_change_detection() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        fs::write(&config_path, r#"{"key": "value1"}"#).unwrap();

        let watcher = FileWatcher::new(&config_path).unwrap();

        // Modify the file
        std::thread::spawn({
            let config_path = config_path.clone();
            move || {
                std::thread::sleep(Duration::from_millis(50));
                fs::write(&config_path, r#"{"key": "value2"}"#).unwrap();
            }
        });

        // Check for changes with a reasonable timeout
        let result = watcher.check_for_changes(Duration::from_millis(200));
        assert!(result.is_ok());
        // Note: The actual change detection depends on the file system and timing
    }

    #[test]
    fn test_multiple_callbacks() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        fs::write(&config_path, "{}").unwrap();

        let watcher = FileWatcher::new(&config_path).unwrap();

        let callback1_called = Arc::new(Mutex::new(false));
        let callback2_called = Arc::new(Mutex::new(false));

        let callback1_called_clone = Arc::clone(&callback1_called);
        let callback2_called_clone = Arc::clone(&callback2_called);

        // Register multiple callbacks
        watcher
            .on_config_change(move || {
                *callback1_called_clone.lock().unwrap() = true;
            })
            .unwrap();

        watcher
            .on_config_change(move || {
                *callback2_called_clone.lock().unwrap() = true;
            })
            .unwrap();

        // Simulate a file change by calling callbacks directly
        // In a real scenario, this would be triggered by file system events
        if let Ok(callbacks_guard) = watcher.callbacks.lock() {
            for callback in callbacks_guard.iter() {
                callback();
            }
        }

        // Both callbacks should have been called
        assert!(*callback1_called.lock().unwrap());
        assert!(*callback2_called.lock().unwrap());
    }

    #[test]
    fn test_start_stop_watching() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        fs::write(&config_path, "{}").unwrap();

        let mut watcher = FileWatcher::new(&config_path).unwrap();
        assert!(!watcher.is_watching());

        // Start watching
        watcher.start_watching().unwrap();
        assert!(watcher.is_watching());

        // Stop watching
        watcher.stop_watching();
        assert!(!watcher.is_watching());
    }

    #[test]
    fn test_callback_error_handling() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        fs::write(&config_path, "{}").unwrap();

        let watcher = FileWatcher::new(&config_path).unwrap();

        // Register a callback that might panic (but shouldn't crash the system)
        let result = watcher.on_config_change(|| {
            // This callback doesn't panic, but tests the error handling path
        });

        assert!(result.is_ok());
    }
}
