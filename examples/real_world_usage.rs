//! Real-world usage examples demonstrating common patterns and best practices.
//!
//! This example shows how to use spice_instance-rust in a production-like application
//! with proper error handling, validation, and configuration management.
//!
//! Run with: cargo run --example real_world_usage --features cli

use serde::{Deserialize, Serialize};
use spicex::{ConfigError, ConfigValue, Spice};
use std::collections::HashMap;
use std::time::Duration;

/// Application configuration structure
#[derive(Debug, Deserialize, Serialize, Clone)]
struct AppConfig {
    /// Application metadata
    app: AppInfo,
    /// Database configuration
    database: DatabaseConfig,
    /// Server configuration
    server: ServerConfig,
    /// Logging configuration
    logging: LoggingConfig,
    /// Feature flags
    #[serde(default)]
    features: FeatureFlags,
    /// External services
    #[serde(default)]
    services: HashMap<String, ServiceConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct AppInfo {
    name: String,
    version: String,
    #[serde(default)]
    environment: String,
    #[serde(default)]
    debug: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct DatabaseConfig {
    host: String,
    port: u16,
    #[serde(default)]
    database: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    password: String,
    #[serde(default)]
    ssl: bool,
    #[serde(default = "default_max_connections")]
    max_connections: u32,
    #[serde(default = "default_timeout")]
    timeout_seconds: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ServerConfig {
    host: String,
    port: u16,
    #[serde(default = "default_workers")]
    workers: u32,
    #[serde(default)]
    tls: Option<TlsConfig>,
    #[serde(default = "default_request_timeout")]
    request_timeout_ms: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct TlsConfig {
    cert_file: String,
    key_file: String,
    #[serde(default)]
    ca_file: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct LoggingConfig {
    #[serde(default = "default_log_level")]
    level: String,
    #[serde(default)]
    format: String,
    #[serde(default)]
    file: Option<String>,
    #[serde(default)]
    rotate: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
struct FeatureFlags {
    #[serde(default)]
    auth_enabled: bool,
    #[serde(default)]
    metrics_enabled: bool,
    #[serde(default)]
    tracing_enabled: bool,
    #[serde(default)]
    rate_limiting: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ServiceConfig {
    url: String,
    #[serde(default = "default_service_timeout")]
    timeout_ms: u64,
    #[serde(default)]
    retries: u32,
    #[serde(default)]
    headers: HashMap<String, String>,
}

// Default value functions
fn default_max_connections() -> u32 {
    100
}
fn default_timeout() -> u64 {
    30
}
fn default_workers() -> u32 {
    4
}
fn default_request_timeout() -> u64 {
    30000
}
fn default_log_level() -> String {
    "info".to_string()
}
fn default_service_timeout() -> u64 {
    5000
}

impl AppConfig {
    /// Validates the configuration and returns detailed errors
    fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Validate app info
        if self.app.name.is_empty() {
            errors.push("app.name cannot be empty".to_string());
        }
        if self.app.version.is_empty() {
            errors.push("app.version cannot be empty".to_string());
        }

        // Validate database config
        if self.database.host.is_empty() {
            errors.push("database.host cannot be empty".to_string());
        }
        if self.database.port == 0 {
            errors.push("database.port must be greater than 0".to_string());
        }
        if self.database.max_connections == 0 {
            errors.push("database.max_connections must be greater than 0".to_string());
        }

        // Validate server config
        if self.server.port == 0 {
            errors.push("server.port must be greater than 0".to_string());
        }
        if self.server.workers == 0 {
            errors.push("server.workers must be greater than 0".to_string());
        }

        // Validate TLS config if present
        if let Some(ref tls) = self.server.tls {
            if tls.cert_file.is_empty() {
                errors.push("server.tls.cert_file cannot be empty when TLS is enabled".to_string());
            }
            if tls.key_file.is_empty() {
                errors.push("server.tls.key_file cannot be empty when TLS is enabled".to_string());
            }
        }

        // Validate logging config
        let valid_log_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_log_levels.contains(&self.logging.level.as_str()) {
            errors.push(format!(
                "logging.level must be one of: {}",
                valid_log_levels.join(", ")
            ));
        }

        // Validate service configs
        for (name, service) in &self.services {
            if service.url.is_empty() {
                errors.push(format!("services.{}.url cannot be empty", name));
            }
            if !service.url.starts_with("http://") && !service.url.starts_with("https://") {
                errors.push(format!("services.{}.url must be a valid HTTP(S) URL", name));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Returns the database connection timeout as a Duration
    pub fn database_timeout(&self) -> Duration {
        Duration::from_secs(self.database.timeout_seconds)
    }

    /// Returns the server request timeout as a Duration
    pub fn request_timeout(&self) -> Duration {
        Duration::from_millis(self.server.request_timeout_ms)
    }

    /// Checks if a feature is enabled
    pub fn is_feature_enabled(&self, feature: &str) -> bool {
        match feature {
            "auth" => self.features.auth_enabled,
            "metrics" => self.features.metrics_enabled,
            "tracing" => self.features.tracing_enabled,
            "rate_limiting" => self.features.rate_limiting,
            _ => false,
        }
    }

    /// Gets service configuration by name
    pub fn get_service(&self, name: &str) -> Option<&ServiceConfig> {
        self.services.get(name)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Real-World SPICE Usage Example ===\n");

    // Initialize configuration manager
    let config = initialize_configuration()?;

    // Demonstrate configuration usage
    demonstrate_configuration_usage(&config)?;

    // Show configuration management features
    demonstrate_advanced_features()?;

    Ok(())
}

/// Initializes the configuration with proper error handling and validation
fn initialize_configuration() -> Result<AppConfig, Box<dyn std::error::Error>> {
    println!("🔧 Initializing configuration...");

    let mut spice_instance = Spice::new();

    // 1. Set up comprehensive defaults
    setup_defaults(&mut spice_instance)?;

    // 2. Configure file discovery
    setup_file_discovery(&mut spice_instance);

    // 3. Load configuration files
    load_configuration_files(&mut spice_instance)?;

    // 4. Set up environment variable support
    setup_environment_variables(&mut spice_instance);

    // 5. Process command line arguments
    #[cfg(feature = "cli")]
    process_command_line_arguments(&mut spice_instance)?;

    // 6. Deserialize and validate configuration
    let config = deserialize_and_validate_config(&spice_instance)?;

    // 7. Set up file watching for hot reloading
    setup_file_watching(&mut spice_instance)?;

    println!("✅ Configuration initialized successfully\n");
    Ok(config)
}

/// Sets up comprehensive default values
fn setup_defaults(spice_instance: &mut Spice) -> Result<(), ConfigError> {
    println!("  📋 Setting up defaults...");

    // App defaults
    spice_instance.set_default("app.name", ConfigValue::from("MyApp"))?;
    spice_instance.set_default("app.version", ConfigValue::from("1.0.0"))?;
    spice_instance.set_default("app.environment", ConfigValue::from("development"))?;
    spice_instance.set_default("app.debug", ConfigValue::from(false))?;

    // Database defaults
    spice_instance.set_default("database.host", ConfigValue::from("localhost"))?;
    spice_instance.set_default("database.port", ConfigValue::from(5432i64))?;
    spice_instance.set_default("database.database", ConfigValue::from("myapp"))?;
    spice_instance.set_default("database.username", ConfigValue::from("postgres"))?;
    spice_instance.set_default("database.password", ConfigValue::from(""))?;
    spice_instance.set_default("database.ssl", ConfigValue::from(false))?;
    spice_instance.set_default("database.max_connections", ConfigValue::from(100i64))?;
    spice_instance.set_default("database.timeout_seconds", ConfigValue::from(30i64))?;

    // Server defaults
    spice_instance.set_default("server.host", ConfigValue::from("0.0.0.0"))?;
    spice_instance.set_default("server.port", ConfigValue::from(8080i64))?;
    spice_instance.set_default("server.workers", ConfigValue::from(4i64))?;
    spice_instance.set_default("server.request_timeout_ms", ConfigValue::from(30000i64))?;

    // Logging defaults
    spice_instance.set_default("logging.level", ConfigValue::from("info"))?;
    spice_instance.set_default("logging.format", ConfigValue::from("json"))?;
    spice_instance.set_default("logging.rotate", ConfigValue::from(true))?;

    // Feature flags defaults
    spice_instance.set_default("features.auth_enabled", ConfigValue::from(true))?;
    spice_instance.set_default("features.metrics_enabled", ConfigValue::from(true))?;
    spice_instance.set_default("features.tracing_enabled", ConfigValue::from(false))?;
    spice_instance.set_default("features.rate_limiting", ConfigValue::from(false))?;

    println!("     ✓ Default values configured");
    Ok(())
}

/// Configures file discovery paths and naming
fn setup_file_discovery(spice_instance: &mut Spice) {
    println!("  📁 Setting up file discovery...");

    spice_instance.set_config_name("config");

    // Add multiple search paths in order of preference
    spice_instance.add_config_path("."); // Current directory
    spice_instance.add_config_path("./config"); // Config subdirectory
    spice_instance.add_config_path("./configs"); // Configs subdirectory
    spice_instance.add_config_path("/etc/myapp"); // System config directory

    // Add user-specific config directory
    if let Some(home) = dirs::home_dir() {
        spice_instance.add_config_path(home.join(".config/myapp"));
        spice_instance.add_config_path(home.join(".myapp"));
    }

    println!("     ✓ File discovery paths configured");
}

/// Loads configuration files with proper error handling
fn load_configuration_files(spice_instance: &mut Spice) -> Result<(), ConfigError> {
    println!("  📄 Loading configuration files...");

    match spice_instance.read_in_config() {
        Ok(()) => {
            println!("     ✓ Configuration file loaded successfully");
        }
        Err(ConfigError::KeyNotFound { .. }) => {
            println!("     ⚠ No configuration file found, using defaults");
        }
        Err(e) => {
            println!("     ❌ Error loading configuration file: {}", e);
            return Err(e);
        }
    }

    // Try to merge additional configuration files
    match spice_instance.merge_in_config() {
        Ok(count) if count > 1 => {
            println!("     ✓ Merged {} configuration files", count);
        }
        Ok(_) => {
            // Only one or no files found, already handled above
        }
        Err(e) => {
            println!("     ⚠ Warning merging additional config files: {}", e);
        }
    }

    Ok(())
}

/// Sets up environment variable support
fn setup_environment_variables(spice_instance: &mut Spice) {
    println!("  🌍 Setting up environment variables...");

    spice_instance.set_env_prefix("MYAPP");
    spice_instance.set_automatic_env(true);

    println!("     ✓ Environment variables with MYAPP_ prefix will be used");
    println!("     ✓ Example: MYAPP_DATABASE_HOST -> database.host");
}

/// Processes command line arguments
#[cfg(feature = "cli")]
fn process_command_line_arguments(
    spice_instance: &mut Spice,
) -> Result<(), Box<dyn std::error::Error>> {
    use clap::{Arg, Command};

    println!("  🖥️  Processing command line arguments...");

    let app = Command::new("myapp")
        .version("1.0.0")
        .about("Example application using spice_instance-rust")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
                .action(clap::ArgAction::Set),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .help("Server port")
                .action(clap::ArgAction::Set),
        )
        .arg(
            Arg::new("host")
                .long("host")
                .value_name("HOST")
                .help("Server host")
                .action(clap::ArgAction::Set),
        )
        .arg(
            Arg::new("debug")
                .short('d')
                .long("debug")
                .help("Enable debug mode")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("log-level")
                .short('l')
                .long("log-level")
                .value_name("LEVEL")
                .help("Log level (trace, debug, info, warn, error)")
                .action(clap::ArgAction::Set),
        );

    // For demo purposes, simulate some command line arguments
    let args = vec!["myapp", "--port", "9000", "--debug", "--log-level", "debug"];

    match app.try_get_matches_from(args) {
        Ok(matches) => {
            // Create custom mappings for better configuration key names
            let mut mappings = HashMap::new();
            mappings.insert("port".to_string(), "server.port".to_string());
            mappings.insert("host".to_string(), "server.host".to_string());
            mappings.insert("debug".to_string(), "app.debug".to_string());
            mappings.insert("log-level".to_string(), "logging.level".to_string());

            spice_instance.bind_flags_with_mappings(matches, mappings);
            println!("     ✓ Command line arguments processed");
        }
        Err(e) => {
            println!("     ⚠ Error processing CLI arguments: {}", e);
        }
    }

    Ok(())
}

/// Deserializes and validates the configuration
fn deserialize_and_validate_config(
    spice_instance: &Spice,
) -> Result<AppConfig, Box<dyn std::error::Error>> {
    println!("  🔍 Deserializing and validating configuration...");

    // Deserialize configuration with validation
    let config: AppConfig =
        spice_instance.unmarshal_with_validation(|config: &AppConfig| match config.validate() {
            Ok(()) => Ok(()),
            Err(errors) => {
                let error_msg = format!(
                    "Configuration validation failed:\n  - {}",
                    errors.join("\n  - ")
                );
                Err(ConfigError::invalid_value(error_msg))
            }
        })?;

    println!("     ✓ Configuration deserialized and validated successfully");
    Ok(config)
}

/// Sets up file watching for hot reloading
fn setup_file_watching(spice_instance: &mut Spice) -> Result<(), ConfigError> {
    println!("  👀 Setting up file watching...");

    match spice_instance.watch_config() {
        Ok(()) => {
            // Register callback for configuration changes
            spice_instance.on_config_change(|| {
                println!("🔄 Configuration file changed! Reloading...");
                // In a real application, you would trigger a configuration reload here
            })?;

            println!("     ✓ File watching enabled");
        }
        Err(e) => {
            println!("     ⚠ Could not enable file watching: {}", e);
        }
    }

    Ok(())
}

/// Demonstrates various ways to use the configuration
fn demonstrate_configuration_usage(config: &AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Configuration Usage Examples:\n");

    // 1. Basic configuration access
    println!("1. Basic Configuration Access:");
    println!("   App Name: {}", config.app.name);
    println!("   Version: {}", config.app.version);
    println!("   Environment: {}", config.app.environment);
    println!("   Debug Mode: {}", config.app.debug);
    println!();

    // 2. Database configuration
    println!("2. Database Configuration:");
    println!("   Host: {}", config.database.host);
    println!("   Port: {}", config.database.port);
    println!("   Database: {}", config.database.database);
    println!("   SSL Enabled: {}", config.database.ssl);
    println!("   Max Connections: {}", config.database.max_connections);
    println!("   Timeout: {:?}", config.database_timeout());
    println!();

    // 3. Server configuration
    println!("3. Server Configuration:");
    println!(
        "   Listen Address: {}:{}",
        config.server.host, config.server.port
    );
    println!("   Workers: {}", config.server.workers);
    println!("   Request Timeout: {:?}", config.request_timeout());
    if let Some(ref tls) = config.server.tls {
        println!("   TLS Enabled: Yes");
        println!("   Certificate: {}", tls.cert_file);
        println!("   Private Key: {}", tls.key_file);
    } else {
        println!("   TLS Enabled: No");
    }
    println!();

    // 4. Feature flags
    println!("4. Feature Flags:");
    println!("   Authentication: {}", config.is_feature_enabled("auth"));
    println!("   Metrics: {}", config.is_feature_enabled("metrics"));
    println!("   Tracing: {}", config.is_feature_enabled("tracing"));
    println!(
        "   Rate Limiting: {}",
        config.is_feature_enabled("rate_limiting")
    );
    println!();

    // 5. External services
    println!("5. External Services:");
    if config.services.is_empty() {
        println!("   No external services configured");
    } else {
        for (name, service) in &config.services {
            println!("   {}: {}", name, service.url);
            println!("     Timeout: {}ms", service.timeout_ms);
            println!("     Retries: {}", service.retries);
        }
    }
    println!();

    // 6. Logging configuration
    println!("6. Logging Configuration:");
    println!("   Level: {}", config.logging.level);
    println!("   Format: {}", config.logging.format);
    if let Some(ref file) = config.logging.file {
        println!("   File: {}", file);
    }
    println!("   Rotation: {}", config.logging.rotate);
    println!();

    Ok(())
}

/// Demonstrates advanced configuration management features
fn demonstrate_advanced_features() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Advanced Features Demo:\n");

    let mut spice_instance = Spice::new();

    // 1. Dynamic configuration updates
    println!("1. Dynamic Configuration Updates:");
    spice_instance.set("runtime.feature_x", ConfigValue::from(true))?;
    spice_instance.set("runtime.max_requests", ConfigValue::from(1000i64))?;
    println!("   ✓ Runtime configuration updated");

    // 2. Configuration serialization
    println!("\n2. Configuration Serialization:");

    // Set up some sample configuration
    let mut sample_config = HashMap::new();
    sample_config.insert("app.name".to_string(), ConfigValue::from("Demo App"));
    sample_config.insert("app.version".to_string(), ConfigValue::from("2.0.0"));
    sample_config.insert("server.port".to_string(), ConfigValue::from(8080i64));
    sample_config.insert("debug".to_string(), ConfigValue::from(true));

    spice_instance.set_defaults(sample_config)?;

    // Write configuration to different formats
    match spice_instance.write_config("demo_output.json") {
        Ok(()) => println!("   ✓ Configuration written to demo_output.json"),
        Err(e) => println!("   ❌ Failed to write JSON: {}", e),
    }

    match spice_instance.write_config_as("demo_output.yaml", "yaml") {
        Ok(()) => println!("   ✓ Configuration written to demo_output.yaml"),
        Err(e) => println!("   ❌ Failed to write YAML: {}", e),
    }

    // 3. Sub-configuration access
    println!("\n3. Sub-Configuration Access:");

    // Create nested configuration
    let mut db_config = HashMap::new();
    db_config.insert("host".to_string(), ConfigValue::from("localhost"));
    db_config.insert("port".to_string(), ConfigValue::from(5432i64));
    db_config.insert("ssl".to_string(), ConfigValue::from(true));
    spice_instance.set("database", ConfigValue::Object(db_config))?;

    if let Some(db_viper) = spice_instance.sub("database")? {
        let host = db_viper.get_string("host")?.unwrap_or_default();
        let port = db_viper.get_i64("port")?.unwrap_or(0);
        let ssl = db_viper.get_bool("ssl")?.unwrap_or(false);

        println!("   Database sub-config: {}:{} (SSL: {})", host, port, ssl);
    }

    // 4. Configuration introspection
    println!("\n4. Configuration Introspection:");
    let all_keys = spice_instance.all_keys();
    println!("   Total configuration keys: {}", all_keys.len());
    println!(
        "   Sample keys: {:?}",
        &all_keys[..std::cmp::min(5, all_keys.len())]
    );

    let layer_info = spice_instance.layer_info();
    println!("   Configuration layers:");
    for (i, (source, priority)) in layer_info.iter().enumerate() {
        println!("     {}. {} ({:?})", i + 1, source, priority);
    }

    println!("\n✨ Advanced features demonstration complete!");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_validation() {
        let mut config = AppConfig {
            app: AppInfo {
                name: "TestApp".to_string(),
                version: "1.0.0".to_string(),
                environment: "test".to_string(),
                debug: false,
            },
            database: DatabaseConfig {
                host: "localhost".to_string(),
                port: 5432,
                database: "test".to_string(),
                username: "test".to_string(),
                password: "".to_string(),
                ssl: false,
                max_connections: 10,
                timeout_seconds: 30,
            },
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                workers: 2,
                tls: None,
                request_timeout_ms: 30000,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                file: None,
                rotate: false,
            },
            features: FeatureFlags::default(),
            services: HashMap::new(),
        };

        // Valid configuration should pass
        assert!(config.validate().is_ok());

        // Invalid configuration should fail
        config.app.name = "".to_string();
        config.database.port = 0;
        config.logging.level = "invalid".to_string();

        let errors = config.validate().unwrap_err();
        assert!(errors.len() >= 3);
        assert!(errors.iter().any(|e| e.contains("app.name")));
        assert!(errors.iter().any(|e| e.contains("database.port")));
        assert!(errors.iter().any(|e| e.contains("logging.level")));
    }

    #[test]
    fn test_feature_flags() {
        let config = AppConfig {
            app: AppInfo {
                name: "TestApp".to_string(),
                version: "1.0.0".to_string(),
                environment: "test".to_string(),
                debug: false,
            },
            database: DatabaseConfig {
                host: "localhost".to_string(),
                port: 5432,
                database: "test".to_string(),
                username: "test".to_string(),
                password: "".to_string(),
                ssl: false,
                max_connections: 10,
                timeout_seconds: 30,
            },
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                workers: 2,
                tls: None,
                request_timeout_ms: 30000,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                file: None,
                rotate: false,
            },
            features: FeatureFlags {
                auth_enabled: true,
                metrics_enabled: false,
                tracing_enabled: true,
                rate_limiting: false,
            },
            services: HashMap::new(),
        };

        assert!(config.is_feature_enabled("auth"));
        assert!(!config.is_feature_enabled("metrics"));
        assert!(config.is_feature_enabled("tracing"));
        assert!(!config.is_feature_enabled("rate_limiting"));
        assert!(!config.is_feature_enabled("unknown"));
    }
}
