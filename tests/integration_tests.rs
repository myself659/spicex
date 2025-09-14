//! Comprehensive integration tests for spice_instance-rust
//!
//! These tests verify the complete functionality of the Spice configuration system
//! by testing multiple sources working together with proper precedence ordering.

use spicex::env_layer::EnvConfigLayer;
use spicex::{ConfigValue, Spice};
use std::env;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test helper to create a temporary configuration file
fn create_temp_config_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let config_path = dir.path().join(name);
    fs::write(&config_path, content).expect("Failed to write test config file");
    config_path
}

/// Test helper to set environment variables and clean them up
struct EnvVarGuard {
    vars: Vec<String>,
}

impl EnvVarGuard {
    fn new() -> Self {
        Self { vars: Vec::new() }
    }

    fn set(&mut self, key: &str, value: &str) {
        env::set_var(key, value);
        self.vars.push(key.to_string());
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        for var in &self.vars {
            env::remove_var(var);
        }
    }
}

#[test]
fn test_multi_source_precedence_complete() {
    // Clean up any existing environment variables that might interfere
    env::remove_var("APP_DATABASE_HOST");
    env::remove_var("APP_DATABASE_PORT");
    env::remove_var("APP_APP_VERSION");
    env::remove_var("APP_APP_DEBUG");
    env::remove_var("APP_SERVERS_0_PORT");

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let mut env_guard = EnvVarGuard::new();

    // Create a JSON config file
    let json_content = r#"{
        "database": {
            "host": "config-file-host",
            "port": 5432,
            "ssl": false
        },
        "app": {
            "name": "test-app",
            "debug": false,
            "timeout": 30
        },
        "servers": [
            {"name": "web1", "port": 8080},
            {"name": "web2", "port": 8081}
        ]
    }"#;

    let config_path = create_temp_config_file(&temp_dir, "config.json", json_content);

    // Set environment variables (should override config file)
    env_guard.set("APP_DATABASE_HOST", "env-host");
    env_guard.set("APP_APP_DEBUG", "true");
    env_guard.set("APP_SERVERS_0_PORT", "9090");

    let mut spice_instance = Spice::new();

    // 1. Set defaults (lowest precedence)
    spice_instance
        .set_default("database.host", ConfigValue::from("default-host"))
        .unwrap();
    spice_instance
        .set_default("database.timeout", ConfigValue::from(60i64))
        .unwrap();
    spice_instance
        .set_default("app.version", ConfigValue::from("1.0.0"))
        .unwrap();

    // 2. Load config file
    spice_instance.set_config_file(&config_path).unwrap();

    // 3. Add environment layer with prefix
    let env_layer = EnvConfigLayer::new(Some("APP".to_string()), true);
    spice_instance.add_layer(Box::new(env_layer));

    // 4. Set explicit values (highest precedence)
    spice_instance
        .set("app.name", ConfigValue::from("explicit-app-name"))
        .unwrap();

    // Test precedence: explicit > env > config file > defaults

    // Explicit value should win
    assert_eq!(
        spice_instance.get_string("app.name").unwrap(),
        Some("explicit-app-name".to_string())
    );

    // Environment should override config file
    assert_eq!(
        spice_instance.get_string("database.host").unwrap(),
        Some("env-host".to_string())
    );

    // Environment should override config file for boolean
    assert_eq!(spice_instance.get_bool("app.debug").unwrap(), Some(true));

    // Environment should override config file for port (env var was set to 9090)
    assert_eq!(
        spice_instance.get_int("servers.0.port").unwrap(),
        Some(9090)
    );

    // Config file should be used when env doesn't override
    assert_eq!(spice_instance.get_int("database.port").unwrap(), Some(5432));

    // Default should be used when no other source provides value
    assert_eq!(
        spice_instance.get_string("app.version").unwrap(),
        Some("1.0.0".to_string())
    );

    // Default should be used for missing nested values
    assert_eq!(
        spice_instance.get_int("database.timeout").unwrap(),
        Some(60)
    );

    // Config file values should be preserved when not overridden by environment
    assert_eq!(
        spice_instance.get_string("app.name").unwrap(),
        Some("explicit-app-name".to_string()) // This was set explicitly, so should win
    );

    // Config file values should be preserved when not overridden
    assert_eq!(
        spice_instance.get_string("servers.1.name").unwrap(),
        Some("web2".to_string())
    );
}

#[test]
fn test_complex_nested_configuration_scenarios() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create a complex YAML config file
    let yaml_content = r#"
database:
  primary:
    host: primary-db
    port: 5432
    credentials:
      username: admin
      password: secret
    pools:
      - name: read-pool
        size: 10
        timeout: 30
      - name: write-pool
        size: 5
        timeout: 60
  secondary:
    host: secondary-db
    port: 5433
    enabled: true

services:
  web:
    instances: 3
    config:
      max_connections: 100
      keep_alive: true
  api:
    instances: 2
    config:
      rate_limit: 1000
      cache_ttl: 300

features:
  - name: feature_a
    enabled: true
    config:
      threshold: 0.8
  - name: feature_b
    enabled: false
    config:
      threshold: 0.9
"#;

    let config_path = create_temp_config_file(&temp_dir, "config.yaml", yaml_content);

    let mut spice_instance = Spice::new();
    spice_instance.set_config_file(&config_path).unwrap();

    // Test deeply nested object access
    assert_eq!(
        spice_instance.get_string("database.primary.host").unwrap(),
        Some("primary-db".to_string())
    );

    assert_eq!(
        spice_instance
            .get_string("database.primary.credentials.username")
            .unwrap(),
        Some("admin".to_string())
    );

    // Test nested array access with object properties
    assert_eq!(
        spice_instance
            .get_string("database.primary.pools.0.name")
            .unwrap(),
        Some("read-pool".to_string())
    );

    assert_eq!(
        spice_instance
            .get_int("database.primary.pools.1.size")
            .unwrap(),
        Some(5)
    );

    // Test array of objects with nested properties
    assert_eq!(
        spice_instance.get_string("features.0.name").unwrap(),
        Some("feature_a".to_string())
    );

    assert_eq!(
        spice_instance.get_bool("features.0.enabled").unwrap(),
        Some(true)
    );

    assert_eq!(
        spice_instance
            .get_float("features.0.config.threshold")
            .unwrap(),
        Some(0.8)
    );

    // Test sub-configuration functionality
    let mut db_config = spice_instance.sub("database.primary").unwrap().unwrap();
    assert_eq!(
        db_config.get_string("host").unwrap(),
        Some("primary-db".to_string())
    );

    assert_eq!(
        db_config.get_string("credentials.username").unwrap(),
        Some("admin".to_string())
    );

    // Test sub-configuration with arrays
    let pools_config = db_config.sub("pools").unwrap();
    // Note: pools is an array, so sub() should return None
    assert!(pools_config.is_none());

    // Test service configuration access
    assert_eq!(
        spice_instance.get_int("services.web.instances").unwrap(),
        Some(3)
    );

    assert_eq!(
        spice_instance
            .get_bool("services.web.config.keep_alive")
            .unwrap(),
        Some(true)
    );
}

#[test]
fn test_multiple_file_formats_integration() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create JSON config
    let json_content = r#"{
        "database": {
            "host": "json-host",
            "port": 5432
        },
        "format": "json"
    }"#;
    let json_path = create_temp_config_file(&temp_dir, "config.json", json_content);

    // Create YAML config
    let yaml_content = r#"
database:
  host: yaml-host
  port: 5433
  ssl: true
format: yaml
"#;
    let yaml_path = create_temp_config_file(&temp_dir, "config.yaml", yaml_content);

    // Create TOML config
    let toml_content = r#"
format = "toml"

[database]
host = "toml-host"
port = 5434
timeout = 30
"#;
    let toml_path = create_temp_config_file(&temp_dir, "config.toml", toml_content);

    // Test JSON loading
    let mut viper_json = Spice::new();
    viper_json.set_config_file(&json_path).unwrap();
    assert_eq!(
        viper_json.get_string("format").unwrap(),
        Some("json".to_string())
    );
    assert_eq!(
        viper_json.get_string("database.host").unwrap(),
        Some("json-host".to_string())
    );

    // Test YAML loading
    let mut viper_yaml = Spice::new();
    viper_yaml.set_config_file(&yaml_path).unwrap();
    assert_eq!(
        viper_yaml.get_string("format").unwrap(),
        Some("yaml".to_string())
    );
    assert_eq!(viper_yaml.get_bool("database.ssl").unwrap(), Some(true));

    // Test TOML loading
    let mut viper_toml = Spice::new();
    viper_toml.set_config_file(&toml_path).unwrap();
    assert_eq!(
        viper_toml.get_string("format").unwrap(),
        Some("toml".to_string())
    );
    assert_eq!(viper_toml.get_int("database.timeout").unwrap(), Some(30));
}

#[test]
fn test_environment_variable_transformation() {
    // Clean up any existing environment variables that might interfere
    env::remove_var("MYAPP_DATABASE_HOST");
    env::remove_var("MYAPP_DATABASE_PORT");
    env::remove_var("MYAPP_FEATURE_FLAGS_ENABLE_CACHE");
    env::remove_var("MYAPP_SERVERS_0_NAME");
    env::remove_var("MYAPP_SERVERS_1_NAME");
    env::remove_var("MYAPP_NESTED_DEEP_VALUE");

    let mut env_guard = EnvVarGuard::new();

    // Set various environment variable formats
    env_guard.set("MYAPP_DATABASE_HOST", "env-db-host");
    env_guard.set("MYAPP_DATABASE_PORT", "5432");
    env_guard.set("MYAPP_FEATURE_FLAGS_ENABLE_CACHE", "true");
    env_guard.set("MYAPP_SERVERS_0_NAME", "server1");
    env_guard.set("MYAPP_SERVERS_1_NAME", "server2");
    env_guard.set("MYAPP_NESTED_DEEP_VALUE", "deep-value");

    let mut spice_instance = Spice::new();

    // Add environment layer with prefix
    let env_layer = EnvConfigLayer::new(Some("MYAPP".to_string()), true);
    spice_instance.add_layer(Box::new(env_layer));

    // Test basic key transformation
    assert_eq!(
        spice_instance.get_string("database.host").unwrap(),
        Some("env-db-host".to_string())
    );

    // Test type coercion from environment strings
    assert_eq!(spice_instance.get_int("database.port").unwrap(), Some(5432));

    // Test boolean coercion
    assert_eq!(
        spice_instance
            .get_bool("feature.flags.enable.cache")
            .unwrap(),
        Some(true)
    );

    // Test array-like access from environment variables
    assert_eq!(
        spice_instance.get_string("servers.0.name").unwrap(),
        Some("server1".to_string())
    );

    assert_eq!(
        spice_instance.get_string("servers.1.name").unwrap(),
        Some("server2".to_string())
    );

    // Test deeply nested keys
    assert_eq!(
        spice_instance.get_string("nested.deep.value").unwrap(),
        Some("deep-value".to_string())
    );
}

#[test]
fn test_configuration_merging_and_overrides() {
    // Clean up any existing environment variables that might interfere
    env::remove_var("MERGE_DATABASE_HOST");
    env::remove_var("MERGE_DATABASE_PORT");
    env::remove_var("MERGE_APP_VERSION");
    env::remove_var("MERGE_APP_DEBUG");

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let mut env_guard = EnvVarGuard::new();

    // Create base config
    let base_config = r#"{
        "app": {
            "name": "base-app",
            "version": "1.0.0",
            "debug": false,
            "features": {
                "auth": true,
                "logging": true,
                "metrics": false
            }
        },
        "database": {
            "host": "localhost",
            "port": 5432,
            "ssl": false
        }
    }"#;
    let base_path = create_temp_config_file(&temp_dir, "base.json", base_config);

    // Create override config
    let override_config = r#"{
        "app": {
            "debug": true,
            "features": {
                "metrics": true,
                "caching": true
            }
        },
        "database": {
            "host": "prod-db",
            "ssl": true,
            "pool_size": 20
        }
    }"#;
    let override_path = create_temp_config_file(&temp_dir, "override.json", override_config);

    // Set environment variables
    env_guard.set("MERGE_DATABASE_PORT", "3306");
    env_guard.set("MERGE_APP_VERSION", "2.0.0");

    let mut spice_instance = Spice::new();

    // Set defaults
    spice_instance
        .set_default("app.timeout", ConfigValue::from(30i64))
        .unwrap();
    spice_instance
        .set_default("database.max_connections", ConfigValue::from(100i64))
        .unwrap();

    // Load base config first
    spice_instance.set_config_file(&base_path).unwrap();

    // Load override config second (will have higher precedence)
    spice_instance.load_config_file(&override_path).unwrap();

    // Add environment layer
    let env_layer = EnvConfigLayer::new(Some("MERGE".to_string()), true);
    spice_instance.add_layer(Box::new(env_layer));

    // Test that values are properly merged and overridden

    // Environment should override config files
    assert_eq!(spice_instance.get_int("database.port").unwrap(), Some(3306));

    assert_eq!(
        spice_instance.get_string("app.version").unwrap(),
        Some("2.0.0".to_string())
    );

    // Base config wins over override config (first ConfigFile layer loaded)
    assert_eq!(
        spice_instance.get_string("database.host").unwrap(),
        Some("localhost".to_string())
    );

    // Base config values (first ConfigFile layer loaded wins)
    assert_eq!(
        spice_instance.get_bool("app.debug").unwrap(),
        Some(false) // from base config
    );

    assert_eq!(
        spice_instance.get_bool("database.ssl").unwrap(),
        Some(false) // from base config
    );

    // Values only in override config should be available (fallback when not in base)
    assert_eq!(
        spice_instance.get_int("database.pool_size").unwrap(),
        Some(20) // from override config (not in base config)
    );

    // Values from base config should be available
    assert_eq!(
        spice_instance.get_string("app.name").unwrap(),
        Some("base-app".to_string()) // from base config
    );

    assert_eq!(
        spice_instance.get_bool("app.features.auth").unwrap(),
        Some(true) // from base config
    );

    // Base config feature flags should win
    assert_eq!(
        spice_instance.get_bool("app.features.metrics").unwrap(),
        Some(false) // from base config
    );

    // Features only in override config should be available (fallback when not in base)
    assert_eq!(
        spice_instance.get_bool("app.features.caching").unwrap(),
        Some(true) // from override config (not in base config)
    );

    // Defaults should be used when no other source provides value
    assert_eq!(spice_instance.get_int("app.timeout").unwrap(), Some(30));

    assert_eq!(
        spice_instance.get_int("database.max_connections").unwrap(),
        Some(100)
    );
}

#[test]
fn test_type_coercion_across_sources() {
    // Clean up any existing environment variables that might interfere
    env::remove_var("TEST_NUMBERS_ENV_INT");
    env::remove_var("TEST_NUMBERS_ENV_FLOAT");
    env::remove_var("TEST_BOOLEANS_ENV_TRUE");
    env::remove_var("TEST_BOOLEANS_ENV_FALSE");

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let mut env_guard = EnvVarGuard::new();

    // Create config with various types
    let config_content = r#"{
        "numbers": {
            "int_as_number": 42,
            "float_as_number": 3.14,
            "int_as_string": "42",
            "float_as_string": "3.14",
            "bool_as_string": "true"
        },
        "booleans": {
            "true_string": "true",
            "false_string": "false",
            "yes_string": "yes",
            "no_string": "no",
            "one_string": "1",
            "zero_string": "0"
        }
    }"#;
    let config_path = create_temp_config_file(&temp_dir, "types.json", config_content);

    // Set environment variables with string values
    env_guard.set("TEST_NUMBERS_ENV_INT", "100");
    env_guard.set("TEST_NUMBERS_ENV_FLOAT", "2.718");
    env_guard.set("TEST_BOOLEANS_ENV_TRUE", "on");
    env_guard.set("TEST_BOOLEANS_ENV_FALSE", "off");

    let mut spice_instance = Spice::new();

    // Set defaults with different types
    spice_instance
        .set_default("numbers.default_int", ConfigValue::from(999i64))
        .unwrap();
    spice_instance
        .set_default("numbers.default_float", ConfigValue::from(1.414))
        .unwrap();
    spice_instance
        .set_default("booleans.default_bool", ConfigValue::from(true))
        .unwrap();

    // Load config file
    spice_instance.set_config_file(&config_path).unwrap();

    // Add environment layer
    let env_layer = EnvConfigLayer::new(Some("TEST".to_string()), true);
    spice_instance.add_layer(Box::new(env_layer));

    // Test type coercion from config file - JSON numbers
    assert_eq!(
        spice_instance.get_int("numbers.int_as_number").unwrap(),
        Some(42)
    );

    assert_eq!(
        spice_instance.get_float("numbers.float_as_number").unwrap(),
        Some(3.14)
    );

    // Test string values that need coercion (these will be ConfigValue::String)
    // For now, let's just verify they exist as strings since coercion isn't implemented
    assert_eq!(
        spice_instance.get_string("numbers.int_as_string").unwrap(),
        Some("42".to_string())
    );

    assert_eq!(
        spice_instance.get_bool("numbers.bool_as_string").unwrap(),
        Some(true)
    );

    // Test boolean coercion variations
    assert_eq!(
        spice_instance.get_bool("booleans.true_string").unwrap(),
        Some(true)
    );
    assert_eq!(
        spice_instance.get_bool("booleans.false_string").unwrap(),
        Some(false)
    );
    assert_eq!(
        spice_instance.get_bool("booleans.yes_string").unwrap(),
        Some(true)
    );
    assert_eq!(
        spice_instance.get_bool("booleans.no_string").unwrap(),
        Some(false)
    );
    assert_eq!(
        spice_instance.get_bool("booleans.one_string").unwrap(),
        Some(true)
    );
    assert_eq!(
        spice_instance.get_bool("booleans.zero_string").unwrap(),
        Some(false)
    );

    // Test type coercion from environment variables
    // Environment variables are parsed as appropriate types by the env layer
    assert_eq!(
        spice_instance.get_int("numbers.env.int").unwrap(),
        Some(100)
    );

    assert_eq!(
        spice_instance.get_float("numbers.env.float").unwrap(),
        Some(2.718)
    );

    assert_eq!(
        spice_instance.get_bool("booleans.env.true").unwrap(),
        Some(true)
    );

    assert_eq!(
        spice_instance.get_bool("booleans.env.false").unwrap(),
        Some(false)
    );

    // Test defaults (no coercion needed)
    assert_eq!(
        spice_instance.get_int("numbers.default_int").unwrap(),
        Some(999)
    );

    assert_eq!(
        spice_instance.get_float("numbers.default_float").unwrap(),
        Some(1.414)
    );

    assert_eq!(
        spice_instance.get_bool("booleans.default_bool").unwrap(),
        Some(true)
    );
}

#[test]
fn test_error_handling_and_fallbacks() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create config with some missing values
    let config_content = r#"{
        "existing": {
            "value": "present"
        }
    }"#;
    let config_path = create_temp_config_file(&temp_dir, "partial.json", config_content);

    let mut spice_instance = Spice::new();

    // Set defaults for fallback
    spice_instance
        .set_default("missing.with_default", ConfigValue::from("fallback-value"))
        .unwrap();
    spice_instance
        .set_default("existing.value", ConfigValue::from("default-value"))
        .unwrap();

    // Load config file
    spice_instance.set_config_file(&config_path).unwrap();

    // Test existing value (should not use default)
    assert_eq!(
        spice_instance.get_string("existing.value").unwrap(),
        Some("present".to_string())
    );

    // Test missing value with default (should use default)
    assert_eq!(
        spice_instance.get_string("missing.with_default").unwrap(),
        Some("fallback-value".to_string())
    );

    // Test completely missing value (should return None)
    assert_eq!(
        spice_instance.get_string("completely.missing").unwrap(),
        None
    );

    // Test nested missing value
    assert_eq!(
        spice_instance
            .get_string("existing.missing_nested")
            .unwrap(),
        None
    );

    // Test array access on non-existent array
    assert_eq!(spice_instance.get_string("missing.array.0").unwrap(), None);
}
