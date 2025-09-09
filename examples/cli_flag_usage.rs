//! Example demonstrating command line flag integration with Spice .
//!
//! This example shows how to bind command line flags to configuration keys
//! and how they integrate with other configuration sources.

use clap::{Arg, Command};
use spice::{ConfigValue, Spice};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a CLI application with various flag types
    let app = Command::new("cli-example")
        .about("Example CLI application with Spice configuration")
        .disable_help_flag(true)
        .arg(
            Arg::new("host")
                .long("host")
                .short('h')
                .value_name("HOST")
                .help("Database host")
                .action(clap::ArgAction::Set),
        )
        .arg(
            Arg::new("port")
                .long("port")
                .short('p')
                .value_name("PORT")
                .help("Database port")
                .action(clap::ArgAction::Set),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .help("Enable verbose logging")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("debug")
                .long("debug")
                .short('d')
                .help("Debug level (can be repeated: -ddd)")
                .action(clap::ArgAction::Count),
        )
        .arg(
            Arg::new("include")
                .long("include")
                .short('i')
                .value_name("PATH")
                .help("Include paths (can be repeated)")
                .action(clap::ArgAction::Append),
        );

    // Parse command line arguments
    let matches = app.try_get_matches()?;

    // Create Spice instance and set up configuration
    let mut spice_instance = Spice::new();

    // Set some default values
    spice_instance.set_default(
        "database.host",
        ConfigValue::String("localhost".to_string()),
    )?;
    spice_instance.set_default("database.port", ConfigValue::Integer(5432))?;
    spice_instance.set_default("logging.verbose", ConfigValue::Boolean(false))?;
    spice_instance.set_default("logging.level", ConfigValue::String("info".to_string()))?;

    println!("=== Configuration Before Flag Binding ===");
    println!(
        "Database host: {:?}",
        spice_instance.get_string("database.host")?
    );
    println!(
        "Database port: {:?}",
        spice_instance.get_i64("database.port")?
    );
    println!(
        "Verbose logging: {:?}",
        spice_instance.get_bool("logging.verbose")?
    );
    println!(
        "Debug level: {:?}",
        spice_instance.get_i64("logging.debug")?
    );

    // Bind command line flags with custom mappings
    let mut flag_mappings = HashMap::new();
    flag_mappings.insert("host".to_string(), "database.host".to_string());
    flag_mappings.insert("port".to_string(), "database.port".to_string());
    flag_mappings.insert("verbose".to_string(), "logging.verbose".to_string());
    flag_mappings.insert("debug".to_string(), "logging.debug".to_string());

    spice_instance.bind_flags_with_mappings(matches, flag_mappings);

    println!("\n=== Configuration After Flag Binding ===");
    println!(
        "Database host: {:?}",
        spice_instance.get_string("database.host")?
    );
    println!(
        "Database port: {:?}",
        spice_instance.get_i64("database.port")?
    );
    println!(
        "Verbose logging: {:?}",
        spice_instance.get_bool("logging.verbose")?
    );
    println!(
        "Debug level: {:?}",
        spice_instance.get_i64("logging.debug")?
    );

    // Show include paths if provided
    if let Some(include_paths) = spice_instance.get("include")? {
        println!("Include paths: {:?}", include_paths);
    }

    // Demonstrate precedence by setting an explicit value
    spice_instance.set(
        "database.host",
        ConfigValue::String("explicit-host".to_string()),
    )?;
    println!("\n=== After Explicit Set (Highest Precedence) ===");
    println!(
        "Database host: {:?}",
        spice_instance.get_string("database.host")?
    );

    // Show all configuration keys
    println!("\n=== All Configuration Keys ===");
    let all_keys = spice_instance.all_keys();
    for key in all_keys {
        if let Some(value) = spice_instance.get(&key)? {
            println!("{}: {:?}", key, value);
        }
    }

    // Show layer information
    println!("\n=== Configuration Layer Information ===");
    let layer_info = spice_instance.layer_info();
    for (source, priority) in layer_info {
        println!("{}: {:?}", source, priority);
    }

    Ok(())
}
