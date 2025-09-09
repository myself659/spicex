//! Example demonstrating default value management in SPICE.

use spicex::{ConfigValue, Spice};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== SPICE Default Values Example ===\n");

    let mut spice_instance = Spice::new();

    // Set individual default values
    println!("1. Setting individual default values:");
    spice_instance.set_default("database.host", ConfigValue::from("localhost"))?;
    spice_instance.set_default("database.port", ConfigValue::from(5432i64))?;
    spice_instance.set_default("database.ssl", ConfigValue::from(false))?;

    println!(
        "   database.host = {}",
        spice_instance
            .get_string("database.host")?
            .unwrap_or_default()
    );
    println!(
        "   database.port = {}",
        spice_instance.get_i64("database.port")?.unwrap_or_default()
    );
    println!(
        "   database.ssl = {}",
        spice_instance.get_bool("database.ssl")?.unwrap_or_default()
    );

    // Set bulk default values
    println!("\n2. Setting bulk default values:");
    let mut server_defaults = HashMap::new();
    server_defaults.insert("server.host".to_string(), ConfigValue::from("0.0.0.0"));
    server_defaults.insert("server.port".to_string(), ConfigValue::from(8080i64));
    server_defaults.insert("server.timeout".to_string(), ConfigValue::from(30i64));
    server_defaults.insert("server.debug".to_string(), ConfigValue::from(false));

    spice_instance.set_defaults(server_defaults)?;

    println!(
        "   server.host = {}",
        spice_instance
            .get_string("server.host")?
            .unwrap_or_default()
    );
    println!(
        "   server.port = {}",
        spice_instance.get_i64("server.port")?.unwrap_or_default()
    );
    println!(
        "   server.timeout = {}",
        spice_instance
            .get_i64("server.timeout")?
            .unwrap_or_default()
    );
    println!(
        "   server.debug = {}",
        spice_instance.get_bool("server.debug")?.unwrap_or_default()
    );

    // Demonstrate precedence - defaults have lowest priority
    println!("\n3. Demonstrating precedence (defaults vs explicit values):");
    println!(
        "   Before override - database.host = {}",
        spice_instance
            .get_string("database.host")?
            .unwrap_or_default()
    );

    // Override default with explicit value (higher precedence)
    spice_instance.set(
        "database.host",
        ConfigValue::from("production-db.example.com"),
    )?;
    println!(
        "   After override - database.host = {}",
        spice_instance
            .get_string("database.host")?
            .unwrap_or_default()
    );

    // Show all configuration keys
    println!("\n4. All configuration keys:");
    let all_keys = spice_instance.all_keys();
    for key in &all_keys {
        if let Some(value) = spice_instance.get_string(key)? {
            println!("   {} = {}", key, value);
        }
    }

    // Show layer information
    println!("\n5. Configuration layer information:");
    let layer_info = spice_instance.layer_info();
    for (name, priority) in layer_info {
        println!("   Layer: {} (Priority: {:?})", name, priority);
    }

    println!("\n=== Example completed successfully! ===");
    Ok(())
}
