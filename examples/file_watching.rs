//! File watching example for SPICE configuration library.

use spicex::Spice;
use std::fs;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("SPICE - File Watching Example");
    println!("==================================");

    // Create a temporary directory for our config file
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("config.json");

    // Create initial configuration file
    let initial_config = r#"{
    "database": {
        "host": "localhost",
        "port": 5432
    },
    "debug": false
}"#;
    fs::write(&config_path, initial_config)?;
    println!("✓ Created initial config file: {}", config_path.display());

    // Create a new Spice instance and load the config
    let mut spice_instance = Spice::new();
    spice_instance.set_config_file(&config_path)?;
    println!("✓ Loaded configuration file");

    // Display initial values
    println!("\nInitial configuration values:");
    println!(
        "  database.host: {:?}",
        spice_instance.get_string("database.host")?
    );
    println!(
        "  database.port: {:?}",
        spice_instance.get_i64("database.port")?
    );
    println!("  debug: {:?}", spice_instance.get_bool("debug")?);

    // Set up file watching
    spice_instance.watch_config()?;
    println!("✓ Enabled file watching");

    // Register a callback for configuration changes
    let change_count = Arc::new(Mutex::new(0));
    let change_count_clone = Arc::clone(&change_count);

    spice_instance.on_config_change(move || {
        let mut count = change_count_clone.lock().unwrap();
        *count += 1;
        println!("🔄 Configuration change detected! (Change #{}) ", *count);
    })?;
    println!("✓ Registered change callback");

    println!("\n📝 File watching is now active. The configuration will automatically reload when the file changes.");
    println!(
        "   Watched files: {:?}",
        spice_instance.watched_config_files()
    );
    println!("   Is watching: {}", spice_instance.is_watching());

    // Simulate file changes in a separate thread
    let config_path_clone = config_path.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(500));

        // First change
        let updated_config1 = r#"{
    "database": {
        "host": "remote-server",
        "port": 5432
    },
    "debug": true
}"#;
        if let Err(e) = fs::write(&config_path_clone, updated_config1) {
            eprintln!("Failed to write updated config: {}", e);
        } else {
            println!("📝 Updated config file (change 1)");
        }

        thread::sleep(Duration::from_millis(500));

        // Second change
        let updated_config2 = r#"{
    "database": {
        "host": "production-server",
        "port": 3306
    },
    "debug": false,
    "new_feature": "enabled"
}"#;
        if let Err(e) = fs::write(&config_path_clone, updated_config2) {
            eprintln!("Failed to write updated config: {}", e);
        } else {
            println!("📝 Updated config file (change 2)");
        }
    });

    // Wait for changes to be detected
    println!("\n⏳ Waiting for file changes to be detected...");
    thread::sleep(Duration::from_secs(2));

    // Check the change count
    let final_count = *change_count.lock().unwrap();
    println!("\n📊 Total configuration changes detected: {}", final_count);

    // Stop watching
    spice_instance.stop_watching();
    println!("✓ Stopped file watching");
    println!("   Is watching: {}", spice_instance.is_watching());

    println!("\n✅ File watching example completed successfully!");

    Ok(())
}
