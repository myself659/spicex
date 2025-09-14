//! Real-world example: Web server configuration
//!
//! This example demonstrates how to configure a web server application using
//! multiple configuration sources with proper precedence and validation.

use serde::{Deserialize, Serialize};
use spicex::{ConfigValue, EnvConfigLayer, Spice};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ServerConfig {
    host: String,
    port: u16,
    #[serde(default = "default_workers")]
    workers: u32,
    #[serde(default)]
    ssl: SslConfig,
    #[serde(default)]
    timeouts: TimeoutConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct SslConfig {
    #[serde(default)]
    enabled: bool,
    cert_file: Option<String>,
    key_file: Option<String>,
}

impl Default for SslConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cert_file: None,
            key_file: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct TimeoutConfig {
    #[serde(default = "default_read_timeout")]
    read: u64,
    #[serde(default = "default_write_timeout")]
    write: u64,
    #[serde(default = "default_idle_timeout")]
    idle: u64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            read: default_read_timeout(),
            write: default_write_timeout(),
            idle: default_idle_timeout(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct DatabaseConfig {
    host: String,
    port: u16,
    database: String,
    username: String,
    password: String,
    #[serde(default = "default_max_connections")]
    max_connections: u32,
    #[serde(default = "default_connection_timeout")]
    connection_timeout: u64,
    #[serde(default)]
    ssl_mode: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct LoggingConfig {
    #[serde(default = "default_log_level")]
    level: String,
    #[serde(default)]
    format: String,
    file: Option<String>,
    #[serde(default)]
    rotation: LogRotationConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct LogRotationConfig {
    #[serde(default)]
    enabled: bool,
    #[serde(default = "default_max_size")]
    max_size: String,
    #[serde(default = "default_max_files")]
    max_files: u32,
}

impl Default for LogRotationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_size: default_max_size(),
            max_files: default_max_files(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct RedisConfig {
    host: String,
    port: u16,
    #[serde(default)]
    password: Option<String>,
    #[serde(default)]
    database: u32,
    #[serde(default = "default_pool_size")]
    pool_size: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct AppConfig {
    #[serde(default = "default_app_name")]
    name: String,
    #[serde(default = "default_version")]
    version: String,
    #[serde(default)]
    debug: bool,
    #[serde(default = "default_environment")]
    environment: String,

    server: ServerConfig,
    database: DatabaseConfig,
    #[serde(default)]
    logging: LoggingConfig,
    redis: Option<RedisConfig>,

    #[serde(default)]
    features: HashMap<String, bool>,
    #[serde(default)]
    metrics: MetricsConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct MetricsConfig {
    #[serde(default)]
    enabled: bool,
    #[serde(default = "default_metrics_port")]
    port: u16,
    #[serde(default = "default_metrics_path")]
    path: String,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: default_metrics_port(),
            path: default_metrics_path(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: "json".to_string(),
            file: None,
            rotation: LogRotationConfig::default(),
        }
    }
}

// Default value functions
fn default_workers() -> u32 {
    4
}
fn default_read_timeout() -> u64 {
    30
}
fn default_write_timeout() -> u64 {
    30
}
fn default_idle_timeout() -> u64 {
    120
}
fn default_max_connections() -> u32 {
    100
}
fn default_connection_timeout() -> u64 {
    30
}
fn default_log_level() -> String {
    "info".to_string()
}
fn default_max_size() -> String {
    "100MB".to_string()
}
fn default_max_files() -> u32 {
    10
}
fn default_pool_size() -> u32 {
    10
}
fn default_app_name() -> String {
    "web-server".to_string()
}
fn default_version() -> String {
    "1.0.0".to_string()
}
fn default_environment() -> String {
    "development".to_string()
}
fn default_metrics_port() -> u16 {
    9090
}
fn default_metrics_path() -> String {
    "/metrics".to_string()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Web Server Configuration Example");
    println!("===================================");

    // Create a comprehensive configuration setup
    let mut spice_instance = Spice::new();

    // 1. Set up defaults for all configuration values
    setup_defaults(&mut spice_instance)?;

    // 2. Try to load configuration files in order of precedence
    load_config_files(&mut spice_instance)?;

    // 3. Set up environment variable bindings
    setup_environment_variables(&mut spice_instance)?;

    // 4. Override with command line arguments (if any)
    // In a real application, you would parse command line args here

    // 5. Validate and load the complete configuration
    let config: AppConfig = spice_instance.unmarshal()?;

    // 6. Display the final configuration
    display_configuration(&config);

    // 7. Demonstrate accessing individual configuration values
    demonstrate_individual_access(&mut spice_instance)?;

    // 8. Show configuration watching (if files are available)
    demonstrate_config_watching(&mut spice_instance)?;

    println!("\n✅ Web server configuration example completed successfully!");
    Ok(())
}

fn setup_defaults(spice_instance: &mut Spice) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📋 Setting up default configuration values...");

    // Server defaults
    spice_instance.set_default("server.host", ConfigValue::from("localhost"))?;
    spice_instance.set_default("server.port", ConfigValue::from(8080i64))?;
    spice_instance.set_default("server.workers", ConfigValue::from(4i64))?;
    spice_instance.set_default("server.ssl.enabled", ConfigValue::from(false))?;
    spice_instance.set_default("server.timeouts.read", ConfigValue::from(30i64))?;
    spice_instance.set_default("server.timeouts.write", ConfigValue::from(30i64))?;
    spice_instance.set_default("server.timeouts.idle", ConfigValue::from(120i64))?;

    // Database defaults
    spice_instance.set_default("database.host", ConfigValue::from("localhost"))?;
    spice_instance.set_default("database.port", ConfigValue::from(5432i64))?;
    spice_instance.set_default("database.database", ConfigValue::from("myapp"))?;
    spice_instance.set_default("database.username", ConfigValue::from("postgres"))?;
    spice_instance.set_default("database.password", ConfigValue::from("password"))?;
    spice_instance.set_default("database.max_connections", ConfigValue::from(100i64))?;
    spice_instance.set_default("database.connection_timeout", ConfigValue::from(30i64))?;
    spice_instance.set_default("database.ssl_mode", ConfigValue::from("prefer"))?;

    // Logging defaults
    spice_instance.set_default("logging.level", ConfigValue::from("info"))?;
    spice_instance.set_default("logging.format", ConfigValue::from("json"))?;
    spice_instance.set_default("logging.rotation.enabled", ConfigValue::from(false))?;
    spice_instance.set_default("logging.rotation.max_size", ConfigValue::from("100MB"))?;
    spice_instance.set_default("logging.rotation.max_files", ConfigValue::from(10i64))?;

    // Application defaults
    spice_instance.set_default("name", ConfigValue::from("web-server"))?;
    spice_instance.set_default("version", ConfigValue::from("1.0.0"))?;
    spice_instance.set_default("debug", ConfigValue::from(false))?;
    spice_instance.set_default("environment", ConfigValue::from("development"))?;

    // Metrics defaults
    spice_instance.set_default("metrics.enabled", ConfigValue::from(false))?;
    spice_instance.set_default("metrics.port", ConfigValue::from(9090i64))?;
    spice_instance.set_default("metrics.path", ConfigValue::from("/metrics"))?;

    // Feature flags defaults
    spice_instance.set_default("features.auth", ConfigValue::from(true))?;
    spice_instance.set_default("features.rate_limiting", ConfigValue::from(false))?;
    spice_instance.set_default("features.caching", ConfigValue::from(true))?;

    println!("   ✓ Default values configured");
    Ok(())
}

fn load_config_files(spice_instance: &mut Spice) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📁 Loading configuration files...");

    // Set up configuration search paths
    spice_instance.add_config_path(".");
    spice_instance.add_config_path("./config");
    spice_instance.add_config_path("/etc/webserver");

    // Try to load environment-specific config
    let env = env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());

    // Try to load base configuration
    spice_instance.set_config_name("config");
    match spice_instance.read_in_config() {
        Ok(()) => println!("   ✓ Loaded base configuration file"),
        Err(_) => {
            println!("   ⚠ No base configuration file found, using defaults");
            create_sample_config_file()?;
        }
    }

    // Try to load environment-specific configuration
    spice_instance.set_config_name(&format!("config.{}", env));
    match spice_instance.read_in_config() {
        Ok(()) => println!("   ✓ Loaded {}-specific configuration", env),
        Err(_) => println!("   ⚠ No {}-specific configuration found", env),
    }

    Ok(())
}

fn setup_environment_variables(
    spice_instance: &mut Spice,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🌍 Setting up environment variable bindings...");

    // Add environment layer with APP prefix
    let env_layer = EnvConfigLayer::new(Some("APP".to_string()), true);
    spice_instance.add_layer(Box::new(env_layer));

    // Show some example environment variables that would be recognized
    println!("   Environment variables that will be recognized:");
    println!("   - APP_SERVER_HOST -> server.host");
    println!("   - APP_SERVER_PORT -> server.port");
    println!("   - APP_DATABASE_HOST -> database.host");
    println!("   - APP_DATABASE_PASSWORD -> database.password");
    println!("   - APP_DEBUG -> debug");
    println!("   - APP_ENVIRONMENT -> environment");

    // Check if any relevant environment variables are set
    let env_vars = [
        "APP_SERVER_HOST",
        "APP_SERVER_PORT",
        "APP_DATABASE_HOST",
        "APP_DATABASE_PASSWORD",
        "APP_DEBUG",
        "APP_ENVIRONMENT",
    ];

    let mut found_vars = Vec::new();
    for var in &env_vars {
        if let Ok(value) = env::var(var) {
            found_vars.push(format!("{}={}", var, value));
        }
    }

    if found_vars.is_empty() {
        println!("   ⚠ No relevant environment variables found");
    } else {
        println!("   ✓ Found environment variables:");
        for var in found_vars {
            println!("     - {}", var);
        }
    }

    Ok(())
}

fn display_configuration(config: &AppConfig) {
    println!("\n🔧 Final Configuration:");
    println!("======================");

    println!("Application:");
    println!("  Name: {}", config.name);
    println!("  Version: {}", config.version);
    println!("  Environment: {}", config.environment);
    println!("  Debug: {}", config.debug);

    println!("\nServer:");
    println!("  Host: {}", config.server.host);
    println!("  Port: {}", config.server.port);
    println!("  Workers: {}", config.server.workers);
    println!("  SSL Enabled: {}", config.server.ssl.enabled);
    if let Some(cert) = &config.server.ssl.cert_file {
        println!("  SSL Cert: {}", cert);
    }
    println!(
        "  Timeouts: read={}s, write={}s, idle={}s",
        config.server.timeouts.read, config.server.timeouts.write, config.server.timeouts.idle
    );

    println!("\nDatabase:");
    println!("  Host: {}", config.database.host);
    println!("  Port: {}", config.database.port);
    println!("  Database: {}", config.database.database);
    println!("  Username: {}", config.database.username);
    println!("  Password: [REDACTED]");
    println!("  Max Connections: {}", config.database.max_connections);
    println!("  SSL Mode: {}", config.database.ssl_mode);

    println!("\nLogging:");
    println!("  Level: {}", config.logging.level);
    println!("  Format: {}", config.logging.format);
    if let Some(file) = &config.logging.file {
        println!("  File: {}", file);
    }
    println!(
        "  Rotation: enabled={}, max_size={}, max_files={}",
        config.logging.rotation.enabled,
        config.logging.rotation.max_size,
        config.logging.rotation.max_files
    );

    if let Some(redis) = &config.redis {
        println!("\nRedis:");
        println!("  Host: {}", redis.host);
        println!("  Port: {}", redis.port);
        println!("  Database: {}", redis.database);
        println!("  Pool Size: {}", redis.pool_size);
    }

    println!("\nMetrics:");
    println!("  Enabled: {}", config.metrics.enabled);
    if config.metrics.enabled {
        println!("  Port: {}", config.metrics.port);
        println!("  Path: {}", config.metrics.path);
    }

    if !config.features.is_empty() {
        println!("\nFeature Flags:");
        for (feature, enabled) in &config.features {
            println!("  {}: {}", feature, enabled);
        }
    }
}

fn demonstrate_individual_access(spice_instance: &mut Spice) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔍 Demonstrating individual configuration access:");
    println!("================================================");

    // Access individual values with type conversion
    if let Some(host) = spice_instance.get_string("server.host")? {
        println!("Server host: {}", host);
    }

    if let Some(port) = spice_instance.get_int("server.port")? {
        println!("Server port: {}", port);
    }

    if let Some(debug) = spice_instance.get_bool("debug")? {
        println!("Debug mode: {}", debug);
    }

    // Access nested values
    if let Some(db_host) = spice_instance.get_string("database.host")? {
        println!("Database host: {}", db_host);
    }

    // Access with fallback
    let log_level = spice_instance
        .get_string("logging.level")?
        .unwrap_or_else(|| "info".to_string());
    println!("Log level: {}", log_level);

    // Check if optional sections exist
    if spice_instance.get("redis").is_ok() {
        println!("Redis configuration is available");
    } else {
        println!("Redis configuration is not configured");
    }

    Ok(())
}

fn demonstrate_config_watching(
    spice_instance: &mut Spice,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n👀 Configuration watching setup:");
    println!("================================");

    // In a real application, you would set up file watching like this:
    match spice_instance.watch_config() {
        Ok(()) => {
            println!("✓ Configuration file watching enabled");

            // Set up a callback for configuration changes
            spice_instance.on_config_change(Box::new(|| {
                println!("🔄 Configuration file changed! Reloading...");
                // In a real app, you might want to reload specific components
            }))?;

            println!("✓ Change callback registered");
        }
        Err(e) => {
            println!("⚠ Could not enable file watching: {}", e);
        }
    }

    Ok(())
}

fn create_sample_config_file() -> Result<(), Box<dyn std::error::Error>> {
    let sample_config = r#"{
  "name": "my-web-server",
  "version": "1.2.0",
  "environment": "production",
  "debug": false,
  "server": {
    "host": "0.0.0.0",
    "port": 8080,
    "workers": 8,
    "ssl": {
      "enabled": true,
      "cert_file": "/etc/ssl/server.crt",
      "key_file": "/etc/ssl/server.key"
    },
    "timeouts": {
      "read": 60,
      "write": 60,
      "idle": 300
    }
  },
  "database": {
    "host": "db.example.com",
    "port": 5432,
    "database": "production_db",
    "username": "app_user",
    "password": "secure_password",
    "max_connections": 200,
    "connection_timeout": 30,
    "ssl_mode": "require"
  },
  "logging": {
    "level": "warn",
    "format": "json",
    "file": "/var/log/webserver.log",
    "rotation": {
      "enabled": true,
      "max_size": "500MB",
      "max_files": 30
    }
  },
  "redis": {
    "host": "redis.example.com",
    "port": 6379,
    "database": 0,
    "pool_size": 20
  },
  "metrics": {
    "enabled": true,
    "port": 9090,
    "path": "/metrics"
  },
  "features": {
    "auth": true,
    "rate_limiting": true,
    "caching": true,
    "analytics": false
  }
}"#;

    fs::write("config.json", sample_config)?;
    println!("   ✓ Created sample config.json file");
    Ok(())
}
