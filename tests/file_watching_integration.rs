//! Integration tests for file watching functionality

use spicex::Spice;
use std::fs;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn test_file_watching_integration() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create initial config file
    let config_path = temp_dir.path().join("watch_config.json");
    let initial_content = r#"{
        "app": {
            "name": "initial-app",
            "version": "1.0.0"
        },
        "database": {
            "host": "localhost",
            "port": 5432
        }
    }"#;

    fs::write(&config_path, initial_content).expect("Failed to write initial config");

    let mut spice_instance = Spice::new();
    spice_instance.set_config_file(&config_path).unwrap();

    // Verify initial values
    assert_eq!(
        spice_instance.get_string("app.name").unwrap(),
        Some("initial-app".to_string())
    );
    assert_eq!(
        spice_instance.get_string("app.version").unwrap(),
        Some("1.0.0".to_string())
    );

    // Set up change tracking
    let change_count = Arc::new(Mutex::new(0));
    let change_count_clone = Arc::clone(&change_count);

    // Start watching the config file
    spice_instance.watch_config().unwrap();
    spice_instance
        .on_config_change(Box::new(move || {
            let mut count = change_count_clone.lock().unwrap();
            *count += 1;
        }))
        .unwrap();

    // Modify the config file
    let updated_content = r#"{
        "app": {
            "name": "updated-app",
            "version": "2.0.0"
        },
        "database": {
            "host": "updated-host",
            "port": 3306
        }
    }"#;

    // Write the updated content
    fs::write(&config_path, updated_content).expect("Failed to write updated config");

    // Wait for file system events to be processed
    thread::sleep(Duration::from_millis(500));

    // Verify that the configuration was reloaded
    assert_eq!(
        spice_instance.get_string("app.name").unwrap(),
        Some("updated-app".to_string())
    );
    assert_eq!(
        spice_instance.get_string("app.version").unwrap(),
        Some("2.0.0".to_string())
    );
    assert_eq!(
        spice_instance.get_string("database.host").unwrap(),
        Some("updated-host".to_string())
    );
    assert_eq!(spice_instance.get_int("database.port").unwrap(), Some(3306));

    // Verify that the change callback was called
    let count = change_count.lock().unwrap();
    assert!(
        *count > 0,
        "Change callback should have been called at least once"
    );
}

#[test]
fn test_multiple_file_watching() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create multiple config files
    let config1_path = temp_dir.path().join("config1.json");
    let config2_path = temp_dir.path().join("config2.yaml");

    let config1_content = r#"{
        "service1": {
            "enabled": true,
            "port": 8080
        }
    }"#;

    let config2_content = r#"
service2:
  enabled: true
  port: 8081
"#;

    fs::write(&config1_path, config1_content).expect("Failed to write config1");
    fs::write(&config2_path, config2_content).expect("Failed to write config2");

    let mut spice_instance = Spice::new();

    // Load both config files
    spice_instance.load_config_file(&config1_path).unwrap();
    spice_instance.load_config_file(&config2_path).unwrap();

    // Verify initial values
    assert_eq!(
        spice_instance.get_bool("service1.enabled").unwrap(),
        Some(true)
    );
    assert_eq!(spice_instance.get_int("service1.port").unwrap(), Some(8080));
    assert_eq!(
        spice_instance.get_bool("service2.enabled").unwrap(),
        Some(true)
    );
    assert_eq!(spice_instance.get_int("service2.port").unwrap(), Some(8081));

    // Set up change tracking
    let change_count = Arc::new(Mutex::new(0));
    let change_count_clone = Arc::clone(&change_count);

    // Start watching
    spice_instance.watch_config().unwrap();
    spice_instance
        .on_config_change(Box::new(move || {
            let mut count = change_count_clone.lock().unwrap();
            *count += 1;
        }))
        .unwrap();

    // Update config1
    let updated_config1 = r#"{
        "service1": {
            "enabled": false,
            "port": 9080
        }
    }"#;

    fs::write(&config1_path, updated_config1).expect("Failed to update config1");
    thread::sleep(Duration::from_millis(300));

    // Verify config1 changes
    assert_eq!(
        spice_instance.get_bool("service1.enabled").unwrap(),
        Some(false)
    );
    assert_eq!(spice_instance.get_int("service1.port").unwrap(), Some(9080));

    // Verify config2 is unchanged
    assert_eq!(
        spice_instance.get_bool("service2.enabled").unwrap(),
        Some(true)
    );
    assert_eq!(spice_instance.get_int("service2.port").unwrap(), Some(8081));

    // Update config2
    let updated_config2 = r#"
service2:
  enabled: false
  port: 9081
  timeout: 30
"#;

    fs::write(&config2_path, updated_config2).expect("Failed to update config2");
    thread::sleep(Duration::from_millis(300));

    // Verify config2 changes
    assert_eq!(
        spice_instance.get_bool("service2.enabled").unwrap(),
        Some(false)
    );
    assert_eq!(spice_instance.get_int("service2.port").unwrap(), Some(9081));
    assert_eq!(
        spice_instance.get_int("service2.timeout").unwrap(),
        Some(30)
    );

    // Verify that callbacks were called
    let count = change_count.lock().unwrap();
    assert!(
        *count >= 2,
        "Change callback should have been called at least twice"
    );
}

#[test]
fn test_file_watching_with_invalid_content() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let config_path = temp_dir.path().join("invalid_config.json");
    let valid_content = r#"{
        "app": {
            "name": "valid-app"
        }
    }"#;

    fs::write(&config_path, valid_content).expect("Failed to write valid config");

    let mut spice_instance = Spice::new();
    spice_instance.set_config_file(&config_path).unwrap();

    // Verify initial valid state
    assert_eq!(
        spice_instance.get_string("app.name").unwrap(),
        Some("valid-app".to_string())
    );

    // Set up change tracking with more sophisticated logic
    let callback_count = Arc::new(Mutex::new(0));
    let callback_count_clone = Arc::clone(&callback_count);
    let config_change_times = Arc::new(Mutex::new(Vec::new()));
    let config_change_times_clone = Arc::clone(&config_change_times);

    // Start watching
    spice_instance.watch_config().unwrap();
    spice_instance
        .on_config_change(Box::new(move || {
            let mut count = callback_count_clone.lock().unwrap();
            let mut times = config_change_times_clone.lock().unwrap();
            *count += 1;
            times.push(std::time::Instant::now());
        }))
        .unwrap();

    // Write invalid JSON content
    let invalid_content = r#"{
        "app": {
            "name": "invalid-app"
            // Missing closing brace and comma
        }
    }"#;

    fs::write(&config_path, invalid_content).expect("Failed to write invalid config");
    thread::sleep(Duration::from_millis(300));

    // Check callback count after invalid content write - should be 0
    let invalid_callback_count = *callback_count.lock().unwrap();
    
    // Configuration should remain unchanged due to invalid content
    assert_eq!(
        spice_instance.get_string("app.name").unwrap(),
        Some("valid-app".to_string())
    );
    
    // After accessing config with invalid file, callback should still not have been called
    assert_eq!(*callback_count.lock().unwrap(), invalid_callback_count, 
               "Callback should not be triggered for invalid configuration");

    // Write valid content again
    let new_valid_content = r#"{
        "app": {
            "name": "recovered-app"
        }
    }"#;

    fs::write(&config_path, new_valid_content).expect("Failed to write recovered config");
    thread::sleep(Duration::from_millis(300));

    // Configuration should now be updated
    assert_eq!(
        spice_instance.get_string("app.name").unwrap(),
        Some("recovered-app".to_string())
    );
    
    // Now callback should have been called exactly once (for the valid recovery)
    assert_eq!(*callback_count.lock().unwrap(), 1, 
               "Callback should be triggered exactly once for valid configuration recovery");
}
