//! Migration examples showing how to transition from other configuration libraries to spice_instance-rust.
//!
//! This example demonstrates common patterns and how to migrate from:
//! - std::env (environment variables only)
//! - config crate
//! - clap (command line only)
//! - Manual JSON/YAML parsing
//!
//! Run with: cargo run --example migration_guide --features cli

use serde::Deserialize;
use spicex::{ConfigError, ConfigValue, Spice};
use std::collections::HashMap;

#[derive(Deserialize, Debug, PartialEq)]
struct DatabaseConfig {
    host: String,
    port: u16,
    #[serde(default)]
    ssl: bool,
    #[serde(default)]
    max_connections: u32,
}

#[derive(Deserialize, Debug, PartialEq)]
struct ServerConfig {
    host: String,
    port: u16,
    #[serde(default)]
    workers: u32,
}

#[derive(Deserialize, Debug, PartialEq)]
struct AppConfig {
    database: DatabaseConfig,
    server: ServerConfig,
    #[serde(default)]
    debug: bool,
    #[serde(default)]
    log_level: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== SPICE Migration Guide ===\n");

    // Example 1: Migration from std::env
    println!("1. Migration from std::env (environment variables only)");
    migration_from_std_env()?;
    println!();

    // Example 2: Migration from config crate
    println!("2. Migration from config crate");
    migration_from_config_crate()?;
    println!();

    // Example 3: Migration from clap (CLI only)
    #[cfg(feature = "cli")]
    {
        println!("3. Migration from clap (command line only)");
        migration_from_clap()?;
        println!();
    }

    // Example 4: Migration from manual JSON parsing
    println!("4. Migration from manual JSON parsing");
    migration_from_manual_parsing()?;
    println!();

    // Example 5: Complete migration example
    println!("5. Complete migration example (all sources)");
    complete_migration_example()?;

    Ok(())
}

/// Example 1: Migration from std::env
///
/// Old approach: Using std::env directly with manual parsing and defaults
/// New approach: Using spice_instance-rust with automatic type conversion and defaults
fn migration_from_std_env() -> Result<(), Box<dyn std::error::Error>> {
    println!("OLD WAY (std::env):");
    println!("```rust");
    println!("use std::env;");
    println!();
    println!(
        "let host = env::var(\"DATABASE_HOST\").unwrap_or_else(|_| \"localhost\".to_string());"
    );
    println!("let port: u16 = env::var(\"DATABASE_PORT\")");
    println!("    .unwrap_or_else(|_| \"5432\".to_string())");
    println!("    .parse()");
    println!("    .unwrap_or(5432);");
    println!("let ssl = env::var(\"DATABASE_SSL\")");
    println!("    .unwrap_or_else(|_| \"false\".to_string())");
    println!("    .parse::<bool>()");
    println!("    .unwrap_or(false);");
    println!("```");
    println!();

    // Simulate the old way (commented out to avoid actual env var dependency)
    // let host = std::env::var("DATABASE_HOST").unwrap_or_else(|_| "localhost".to_string());
    // let port: u16 = std::env::var("DATABASE_PORT")
    //     .unwrap_or_else(|_| "5432".to_string())
    //     .parse()
    //     .unwrap_or(5432);

    println!("NEW WAY (spice_instance-rust):");
    println!("```rust");
    println!("use spice::{{Spice , ConfigValue}};");
    println!();
    println!("let mut spice_instance = Spice ::new();");
    println!("spice_instance.set_default(\"host\", ConfigValue::from(\"localhost\"))?;");
    println!("spice_instance.set_default(\"port\", ConfigValue::from(5432i64))?;");
    println!("spice_instance.set_default(\"ssl\", ConfigValue::from(false))?;");
    println!("spice_instance.set_env_prefix(\"DATABASE\");");
    println!("spice_instance.set_automatic_env(true);");
    println!();
    println!("let host = spice_instance.get_string(\"host\")?.unwrap_or_default();");
    println!("let port = spice_instance.get_i64(\"port\")?.unwrap_or(5432) as u16;");
    println!("let ssl = spice_instance.get_bool(\"ssl\")?.unwrap_or(false);");
    println!("```");

    // Demonstrate the new way
    let mut spice_instance = Spice::new();
    spice_instance.set_default("host", ConfigValue::from("localhost"))?;
    spice_instance.set_default("port", ConfigValue::from(5432i64))?;
    spice_instance.set_default("ssl", ConfigValue::from(false))?;
    spice_instance.set_env_prefix("DATABASE");
    spice_instance.set_automatic_env(true);

    let host = spice_instance.get_string("host")?.unwrap_or_default();
    let port = spice_instance.get_i64("port")?.unwrap_or(5432) as u16;
    let ssl = spice_instance.get_bool("ssl")?.unwrap_or(false);

    println!();
    println!("Result: host={}, port={}, ssl={}", host, port, ssl);
    println!("Benefits: Automatic type conversion, better error handling, precedence support");

    Ok(())
}

/// Example 2: Migration from config crate
///
/// Shows how to migrate from the popular `config` crate to spice_instance-rust
fn migration_from_config_crate() -> Result<(), Box<dyn std::error::Error>> {
    println!("OLD WAY (config crate):");
    println!("```rust");
    println!("use config::{{Config, File}};");
    println!();
    println!("let settings = Config::builder()");
    println!("    .add_source(File::with_name(\"config\"))");
    println!("    .add_source(config::Environment::with_prefix(\"APP\"))");
    println!("    .build()?;");
    println!();
    println!("let host: String = settings.get(\"database.host\")?;");
    println!("let port: u16 = settings.get(\"database.port\")?;");
    println!("```");
    println!();

    println!("NEW WAY (spice_instance-rust):");
    println!("```rust");
    println!("use spice::Spice ;");
    println!();
    println!("let mut spice_instance = Spice ::new();");
    println!("spice_instance.set_config_name(\"config\");");
    println!("spice_instance.add_config_path(\".\");");
    println!("spice_instance.set_env_prefix(\"APP\");");
    println!("spice_instance.set_automatic_env(true);");
    println!("let _ = spice_instance.read_in_config(); // Ignore errors, use defaults");
    println!();
    println!("let host = spice_instance.get_string(\"database.host\")?.unwrap_or_default();");
    println!("let port = spice_instance.get_i64(\"database.port\")?.unwrap_or(5432) as u16;");
    println!("```");

    // Demonstrate the new way
    let mut spice_instance = Spice::new();
    spice_instance.set_default("database.host", ConfigValue::from("localhost"))?;
    spice_instance.set_default("database.port", ConfigValue::from(5432i64))?;
    spice_instance.set_config_name("config");
    spice_instance.add_config_path(".");
    spice_instance.set_env_prefix("APP");
    spice_instance.set_automatic_env(true);

    // Try to read config file (will use defaults if not found)
    let _ = spice_instance.read_in_config();

    let host = spice_instance
        .get_string("database.host")?
        .unwrap_or_default();
    let port = spice_instance.get_i64("database.port")?.unwrap_or(5432) as u16;

    println!();
    println!("Result: host={}, port={}", host, port);
    println!("Benefits: Built-in precedence, file watching, better error messages");

    Ok(())
}

/// Example 3: Migration from clap (CLI only)
///
/// Shows how to integrate command line arguments with other configuration sources
#[cfg(feature = "cli")]
fn migration_from_clap() -> Result<(), Box<dyn std::error::Error>> {
    use clap::{Arg, Command};

    println!("OLD WAY (clap only):");
    println!("```rust");
    println!("use clap::{{Arg, Command}};");
    println!();
    println!("let matches = Command::new(\"myapp\")");
    println!("    .arg(Arg::new(\"host\").long(\"host\").default_value(\"localhost\"))");
    println!("    .arg(Arg::new(\"port\").long(\"port\").default_value(\"8080\"))");
    println!("    .get_matches();");
    println!();
    println!("let host = matches.get_one::<String>(\"host\").unwrap();");
    println!("let port: u16 = matches.get_one::<String>(\"port\").unwrap().parse()?;");
    println!("```");
    println!();

    println!("NEW WAY (spice_instance-rust with clap integration):");
    println!("```rust");
    println!("use spice::Spice ;");
    println!("use clap::{{Arg, Command}};");
    println!();
    println!("let app = Command::new(\"myapp\")");
    println!("    .arg(Arg::new(\"host\").long(\"host\"))");
    println!("    .arg(Arg::new(\"port\").long(\"port\"));");
    println!();
    println!("let mut spice_instance = Spice ::new();");
    println!("spice_instance.set_default(\"host\", \"localhost\".into())?;");
    println!("spice_instance.set_default(\"port\", 8080i64.into())?;");
    println!("spice_instance.set_config_name(\"config\");");
    println!("let _ = spice_instance.read_in_config();");
    println!();
    println!("// CLI args override config file and defaults");
    println!("let matches = app.get_matches();");
    println!("spice_instance.bind_flags(matches);");
    println!();
    println!("let host = spice_instance.get_string(\"host\")?.unwrap_or_default();");
    println!("let port = spice_instance.get_i64(\"port\")?.unwrap_or(8080) as u16;");
    println!("```");

    // Demonstrate the new way (simulate CLI args)
    let app = Command::new("myapp")
        .arg(
            Arg::new("host")
                .long("host")
                .value_name("HOST")
                .action(clap::ArgAction::Set),
        )
        .arg(
            Arg::new("port")
                .long("port")
                .value_name("PORT")
                .action(clap::ArgAction::Set),
        );

    let mut spice_instance = Spice::new();
    spice_instance.set_default("host", ConfigValue::from("localhost"))?;
    spice_instance.set_default("port", ConfigValue::from(8080i64))?;
    spice_instance.set_config_name("config");
    let _ = spice_instance.read_in_config();

    // Simulate command line arguments
    let args = vec![
        "myapp",
        "--host",
        "production.example.com",
        "--port",
        "9000",
    ];
    let matches = app.try_get_matches_from(args)?;
    spice_instance.bind_flags(matches);

    let host = spice_instance.get_string("host")?.unwrap_or_default();
    let port = spice_instance.get_i64("port")?.unwrap_or(8080) as u16;

    println!();
    println!("Result: host={}, port={}", host, port);
    println!("Benefits: Unified configuration with precedence, config files + CLI + env vars");

    Ok(())
}

/// Example 4: Migration from manual JSON parsing
///
/// Shows how to replace manual serde JSON parsing with spice_instance-rust
fn migration_from_manual_parsing() -> Result<(), Box<dyn std::error::Error>> {
    println!("OLD WAY (manual JSON parsing):");
    println!("```rust");
    println!("use serde::Deserialize;");
    println!("use std::fs;");
    println!();
    println!("#[derive(Deserialize)]");
    println!("struct Config {{ database: DatabaseConfig, server: ServerConfig }}");
    println!();
    println!("let content = fs::read_to_string(\"config.json\")?;");
    println!("let config: Config = serde_json::from_str(&content)?;");
    println!("```");
    println!();

    println!("NEW WAY (spice_instance-rust with struct deserialization):");
    println!("```rust");
    println!("use spice::Spice ;");
    println!("use serde::Deserialize;");
    println!();
    println!("#[derive(Deserialize)]");
    println!("struct Config {{ database: DatabaseConfig, server: ServerConfig }}");
    println!();
    println!("let mut spice_instance = Spice ::new();");
    println!("spice_instance.set_config_name(\"config\");");
    println!("spice_instance.add_config_path(\".\");");
    println!("spice_instance.read_in_config()?;");
    println!();
    println!("let config: Config = spice_instance.unmarshal()?;");
    println!("```");

    // Demonstrate the new way with defaults
    let mut spice_instance = Spice::new();

    // Set up some default configuration
    let mut db_defaults = HashMap::new();
    db_defaults.insert("host".to_string(), ConfigValue::from("localhost"));
    db_defaults.insert("port".to_string(), ConfigValue::from(5432i64));
    db_defaults.insert("ssl".to_string(), ConfigValue::from(false));
    db_defaults.insert("max_connections".to_string(), ConfigValue::from(100i64));
    spice_instance.set("database", ConfigValue::Object(db_defaults))?;

    let mut server_defaults = HashMap::new();
    server_defaults.insert("host".to_string(), ConfigValue::from("0.0.0.0"));
    server_defaults.insert("port".to_string(), ConfigValue::from(8080i64));
    server_defaults.insert("workers".to_string(), ConfigValue::from(4i64));
    spice_instance.set("server", ConfigValue::Object(server_defaults))?;

    spice_instance.set("debug", ConfigValue::from(false))?;
    spice_instance.set("log_level", ConfigValue::from("info"))?;

    // Try to read config file (will use defaults if not found)
    spice_instance.set_config_name("config");
    spice_instance.add_config_path(".");
    let _ = spice_instance.read_in_config();

    // Deserialize into struct
    let config: AppConfig = spice_instance.unmarshal()?;

    println!();
    println!("Result: {:#?}", config);
    println!("Benefits: Multiple formats, defaults, environment variables, validation");

    Ok(())
}

/// Example 5: Complete migration example
///
/// Shows a complete real-world migration with all configuration sources
fn complete_migration_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("Complete example with all configuration sources:");
    println!();

    let mut spice_instance = Spice::new();

    // 1. Set up defaults (lowest precedence)
    println!("1. Setting up defaults...");
    spice_instance.set_default("database.host", ConfigValue::from("localhost"))?;
    spice_instance.set_default("database.port", ConfigValue::from(5432i64))?;
    spice_instance.set_default("database.ssl", ConfigValue::from(false))?;
    spice_instance.set_default("database.max_connections", ConfigValue::from(100i64))?;

    spice_instance.set_default("server.host", ConfigValue::from("0.0.0.0"))?;
    spice_instance.set_default("server.port", ConfigValue::from(8080i64))?;
    spice_instance.set_default("server.workers", ConfigValue::from(4i64))?;

    spice_instance.set_default("debug", ConfigValue::from(false))?;
    spice_instance.set_default("log_level", ConfigValue::from("info"))?;

    // 2. Try to load configuration file
    println!("2. Looking for configuration files...");
    spice_instance.set_config_name("config");
    spice_instance.add_config_path(".");
    spice_instance.add_config_path("./configs");
    spice_instance.add_config_path("/etc/myapp");

    match spice_instance.read_in_config() {
        Ok(()) => println!("   ✓ Configuration file loaded"),
        Err(e) => println!("   ⚠ No config file found ({}), using defaults", e),
    }

    // 3. Set up environment variable support
    println!("3. Setting up environment variable support...");
    spice_instance.set_env_prefix("MYAPP");
    spice_instance.set_automatic_env(true);
    println!("   ✓ Environment variables with MYAPP_ prefix will override config file");

    // 4. Simulate some environment variables
    println!("4. Simulating environment variables...");
    // In a real app, these would be actual environment variables
    // std::env::set_var("MYAPP_DEBUG", "true");
    // std::env::set_var("MYAPP_DATABASE_HOST", "prod-db.example.com");

    // For demo purposes, we'll set them directly
    spice_instance.set("debug", ConfigValue::from(true))?; // Simulate env var
    println!("   ✓ MYAPP_DEBUG=true (simulated)");

    // 5. Command line flags (highest precedence except explicit calls)
    #[cfg(feature = "cli")]
    {
        println!("5. Processing command line flags...");
        use clap::{Arg, Command};

        let app = Command::new("myapp")
            .arg(
                Arg::new("port")
                    .long("port")
                    .short('p')
                    .value_name("PORT")
                    .action(clap::ArgAction::Set)
                    .help("Server port"),
            )
            .arg(
                Arg::new("verbose")
                    .long("verbose")
                    .short('v')
                    .action(clap::ArgAction::SetTrue)
                    .help("Enable verbose logging"),
            );

        // Simulate CLI args
        let args = vec!["myapp", "--port", "9000", "--verbose"];
        let matches = app.try_get_matches_from(args)?;
        spice_instance.bind_flags(matches);
        println!("   ✓ CLI flags processed: --port 9000 --verbose");
    }

    // 6. Show final configuration
    println!();
    println!("Final configuration (with precedence applied):");

    let config: AppConfig = spice_instance.unmarshal()?;
    println!("{:#?}", config);

    // 7. Show individual value access
    println!();
    println!("Individual value access:");
    println!(
        "  Database host: {}",
        spice_instance
            .get_string("database.host")?
            .unwrap_or_default()
    );
    println!(
        "  Server port: {}",
        spice_instance.get_i64("server.port")?.unwrap_or(8080)
    );
    println!(
        "  Debug mode: {}",
        spice_instance.get_bool("debug")?.unwrap_or(false)
    );

    #[cfg(feature = "cli")]
    {
        println!(
            "  Verbose: {}",
            spice_instance.get_bool("verbose")?.unwrap_or(false)
        );
    }

    // 8. Show configuration sources
    println!();
    println!("Configuration sources (in precedence order):");
    let layer_info = spice_instance.layer_info();
    for (i, (source, priority)) in layer_info.iter().enumerate() {
        println!("  {}. {} ({:?})", i + 1, source, priority);
    }

    println!();
    println!("Migration complete! 🎉");
    println!();
    println!("Key benefits of spice_instance-rust:");
    println!("  ✓ Unified configuration from multiple sources");
    println!("  ✓ Clear precedence hierarchy");
    println!("  ✓ Automatic type conversion and validation");
    println!("  ✓ Support for multiple file formats");
    println!("  ✓ Environment variable mapping");
    println!("  ✓ Command line flag integration");
    println!("  ✓ File watching and hot reloading");
    println!("  ✓ Struct deserialization with serde");
    println!("  ✓ Comprehensive error handling");

    Ok(())
}
