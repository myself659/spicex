//! Example demonstrating configuration file discovery functionality.
//!
//! This example shows how to use Spice's file discovery features to automatically
//! find and load configuration files from standard locations.

use spicex::Spice;
use std::fs;
use tempfile::TempDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Spice  Configuration File Discovery Example ===\n");

    // Create a temporary directory for our example
    let temp_dir = TempDir::new()?;
    let config_dir = temp_dir.path().join("config");
    fs::create_dir_all(&config_dir)?;

    // Create sample configuration files in different formats
    create_sample_configs(&config_dir)?;

    // Example 1: Basic file discovery
    println!("1. Basic File Discovery:");
    basic_file_discovery(&config_dir)?;

    // Example 2: Multiple search paths
    println!("\n2. Multiple Search Paths:");
    multiple_search_paths(&temp_dir)?;

    // Example 3: Automatic configuration loading
    println!("\n3. Automatic Configuration Loading:");
    automatic_loading(&config_dir)?;

    // Example 4: Merging multiple configuration files
    println!("\n4. Merging Multiple Configuration Files:");
    merge_configurations(&config_dir)?;

    // Example 5: Handling missing files gracefully
    println!("\n5. Handling Missing Files:");
    handle_missing_files()?;

    println!("\n=== Example completed successfully! ===");
    Ok(())
}

fn create_sample_configs(config_dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    // Create JSON config
    let json_config = r#"{
    "app": {
        "name": "MyApp",
        "version": "1.0.0",
        "debug": true
    },
    "database": {
        "host": "localhost",
        "port": 5432,
        "name": "myapp_db"
    },
    "features": ["auth", "logging"]
}"#;
    fs::write(config_dir.join("app.json"), json_config)?;

    // Create YAML config with some different values
    let yaml_config = r#"app:
  name: MyApp
  version: 1.0.0
  debug: false  # Different from JSON
  environment: production  # Only in YAML

database:
  host: localhost
  port: 5432
  name: myapp_db
  ssl: true  # Only in YAML

server:
  port: 8080
  timeout: 30
"#;
    fs::write(config_dir.join("app.yaml"), yaml_config)?;

    // Create TOML config
    let toml_config = r#"[app]
name = "MyApp"
version = "1.0.0"
debug = true

[database]
host = "localhost"
port = 5432
name = "myapp_db"

[cache]
enabled = true
ttl = 3600
"#;
    fs::write(config_dir.join("app.toml"), toml_config)?;

    println!("Created sample configuration files:");
    println!("  - {}/app.json", config_dir.display());
    println!("  - {}/app.yaml", config_dir.display());
    println!("  - {}/app.toml", config_dir.display());

    Ok(())
}

fn basic_file_discovery(config_dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut spice_instance = Spice::new();
    spice_instance.set_config_name("app");
    spice_instance.add_config_path(config_dir);

    // Find the first configuration file
    match spice_instance.find_config_file()? {
        Some(config_file) => {
            println!("  Found config file: {}", config_file.display());

            // Load the configuration
            spice_instance.read_in_config()?;

            // Access some values
            if let Some(app_name) = spice_instance.get_string("app.name")? {
                println!("  App name: {}", app_name);
            }
            if let Some(debug) = spice_instance.get_bool("app.debug")? {
                println!("  Debug mode: {}", debug);
            }
        }
        None => {
            println!("  No configuration file found");
        }
    }

    Ok(())
}

fn multiple_search_paths(temp_dir: &TempDir) -> Result<(), Box<dyn std::error::Error>> {
    // Create additional config directories
    let config_dir1 = temp_dir.path().join("config1");
    let config_dir2 = temp_dir.path().join("config2");
    fs::create_dir_all(&config_dir1)?;
    fs::create_dir_all(&config_dir2)?;

    // Create configs with different priorities
    let high_priority_config = r#"{"priority": "high", "source": "config1"}"#;
    let low_priority_config = r#"{"priority": "low", "source": "config2"}"#;

    fs::write(config_dir1.join("priority.json"), high_priority_config)?;
    fs::write(config_dir2.join("priority.json"), low_priority_config)?;

    let mut spice_instance = Spice::new();
    spice_instance.set_config_name("priority");
    spice_instance.add_config_path(&config_dir1); // Higher priority (added first)
    spice_instance.add_config_path(&config_dir2); // Lower priority

    spice_instance.read_in_config()?;

    if let Some(source) = spice_instance.get_string("source")? {
        println!("  Configuration loaded from: {}", source);
        println!("  (Should be 'config1' due to search path priority)");
    }

    Ok(())
}

fn automatic_loading(config_dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut spice_instance = Spice::new();
    spice_instance.set_config_name("app");
    spice_instance.add_config_path(config_dir);

    // Automatically find and load configuration
    match spice_instance.read_in_config() {
        Ok(()) => {
            println!("  Configuration loaded successfully!");

            // Show some loaded values
            if let Some(db_host) = spice_instance.get_string("database.host")? {
                println!("  Database host: {}", db_host);
            }
            if let Some(db_port) = spice_instance.get_i64("database.port")? {
                println!("  Database port: {}", db_port);
            }
        }
        Err(e) => {
            println!("  Failed to load configuration: {}", e);
        }
    }

    Ok(())
}

fn merge_configurations(config_dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut spice_instance = Spice::new();
    spice_instance.set_config_name("app");
    spice_instance.add_config_path(config_dir);

    // Merge all configuration files found
    let merged_count = spice_instance.merge_in_config()?;
    println!("  Merged {} configuration files", merged_count);

    // Show values from different sources
    println!("  Values from merged configuration:");

    if let Some(app_name) = spice_instance.get_string("app.name")? {
        println!("    App name: {}", app_name);
    }

    // This should exist (from YAML)
    if let Some(environment) = spice_instance.get_string("app.environment")? {
        println!("    Environment: {} (from YAML)", environment);
    }

    // This should exist (from YAML)
    if let Some(ssl) = spice_instance.get_bool("database.ssl")? {
        println!("    Database SSL: {} (from YAML)", ssl);
    }

    // This should exist (from TOML)
    if let Some(cache_enabled) = spice_instance.get_bool("cache.enabled")? {
        println!("    Cache enabled: {} (from TOML)", cache_enabled);
    }

    // Show all keys to demonstrate merged configuration
    let all_keys = spice_instance.all_keys();
    println!("  Total configuration keys: {}", all_keys.len());

    Ok(())
}

fn handle_missing_files() -> Result<(), Box<dyn std::error::Error>> {
    let mut spice_instance = Spice::new();
    spice_instance.set_config_name("nonexistent");
    spice_instance.add_config_path("/nonexistent/path");

    match spice_instance.read_in_config() {
        Ok(()) => {
            println!("  Unexpectedly found configuration!");
        }
        Err(e) => {
            println!("  Gracefully handled missing file: {}", e);
        }
    }

    // Try finding files that don't exist
    match spice_instance.find_config_file()? {
        Some(path) => {
            println!("  Unexpectedly found file: {}", path.display());
        }
        None => {
            println!("  Correctly returned None for missing configuration");
        }
    }

    Ok(())
}
