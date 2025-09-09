//! Struct deserialization example for SPICE configuration library.

use serde::{Deserialize, Serialize};
use spicex::{ConfigValue, Spice};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct DatabaseConfig {
    host: String,
    port: u16,
    username: String,
    password: String,
    database: String,
    #[serde(default)]
    ssl: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ServerConfig {
    port: u16,
    host: String,
    debug: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct AppConfig {
    server: ServerConfig,
    database: DatabaseConfig,
}

impl DatabaseConfig {
    fn validate(&self) -> Result<(), String> {
        if self.host.is_empty() {
            return Err("Host cannot be empty".to_string());
        }
        if self.port == 0 {
            return Err("Port cannot be zero".to_string());
        }
        if self.username.is_empty() {
            return Err("Username cannot be empty".to_string());
        }
        if self.password.len() < 6 {
            return Err("Password must be at least 6 characters".to_string());
        }
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("SPICE - Struct Deserialization Example");
    println!("===========================================");

    // Create a new Spice instance
    let mut spice_instance = Spice::new();

    // Set up server configuration
    let mut server_config = HashMap::new();
    server_config.insert("port".to_string(), ConfigValue::from(8080i64));
    server_config.insert("host".to_string(), ConfigValue::from("0.0.0.0"));
    server_config.insert("debug".to_string(), ConfigValue::from(true));
    spice_instance.set("server", ConfigValue::Object(server_config))?;

    // Set up database configuration
    let mut db_config = HashMap::new();
    db_config.insert("host".to_string(), ConfigValue::from("localhost"));
    db_config.insert("port".to_string(), ConfigValue::from(5432i64));
    db_config.insert("username".to_string(), ConfigValue::from("admin"));
    db_config.insert("password".to_string(), ConfigValue::from("secret"));
    db_config.insert("database".to_string(), ConfigValue::from("myapp"));
    // Note: ssl field will use default value (false) since it's not set
    spice_instance.set("database", ConfigValue::Object(db_config))?;

    println!("✓ Set up configuration values");

    // Demonstrate full configuration deserialization
    println!("\n🔄 Deserializing full configuration...");
    let app_config: AppConfig = spice_instance.unmarshal()?;
    println!("✓ Full configuration deserialized:");
    println!(
        "  Server: {}:{} (debug: {})",
        app_config.server.host, app_config.server.port, app_config.server.debug
    );
    println!(
        "  Database: {}@{}:{}/{} (ssl: {})",
        app_config.database.username,
        app_config.database.host,
        app_config.database.port,
        app_config.database.database,
        app_config.database.ssl
    );

    // Demonstrate partial configuration deserialization
    println!("\n🔄 Deserializing database configuration only...");
    let db_config: DatabaseConfig = spice_instance.unmarshal_key("database")?;
    println!("✓ Database configuration deserialized:");
    println!("  Host: {}", db_config.host);
    println!("  Port: {}", db_config.port);
    println!("  Database: {}", db_config.database);
    println!("  SSL: {} (default value)", db_config.ssl);

    // Demonstrate server configuration deserialization
    println!("\n🔄 Deserializing server configuration only...");
    let server_config: ServerConfig = spice_instance.unmarshal_key("server")?;
    println!("✓ Server configuration deserialized:");
    println!("  Host: {}", server_config.host);
    println!("  Port: {}", server_config.port);
    println!("  Debug: {}", server_config.debug);

    // Demonstrate error handling for missing keys
    println!("\n🔄 Testing error handling for missing key...");
    match spice_instance.unmarshal_key::<DatabaseConfig>("nonexistent") {
        Ok(_) => println!("❌ Expected error for missing key"),
        Err(e) => println!("✓ Correctly handled missing key error: {}", e),
    }

    // Demonstrate validation functionality
    println!("\n🔄 Testing validation functionality...");

    // Test successful validation
    let validated_config: DatabaseConfig =
        spice_instance.unmarshal_key_with_validation("database", |config: &DatabaseConfig| {
            config
                .validate()
                .map_err(|e| spicex::ConfigError::invalid_value(e))
        })?;
    println!("✓ Database configuration validated successfully");
    println!("  Validated host: {}", validated_config.host);

    // Test validation failure with invalid configuration
    let mut invalid_viper = Spice::new();
    let mut invalid_db_config = HashMap::new();
    invalid_db_config.insert("host".to_string(), ConfigValue::from("")); // Invalid empty host
    invalid_db_config.insert("port".to_string(), ConfigValue::from(5432i64));
    invalid_db_config.insert("username".to_string(), ConfigValue::from("admin"));
    invalid_db_config.insert("password".to_string(), ConfigValue::from("short")); // Invalid short password
    invalid_db_config.insert("database".to_string(), ConfigValue::from("testdb")); // Add missing database field
    invalid_viper.set("database", ConfigValue::Object(invalid_db_config))?;

    match invalid_viper.unmarshal_key_with_validation::<DatabaseConfig, _>(
        "database",
        |config: &DatabaseConfig| {
            config
                .validate()
                .map_err(|e| spicex::ConfigError::invalid_value(e))
        },
    ) {
        Ok(_) => println!("❌ Expected validation error"),
        Err(e) => println!("✓ Correctly caught validation error: {}", e),
    }

    println!("\n✅ Struct deserialization with validation example completed successfully!");

    Ok(())
}
