//! Integration tests for struct deserialization functionality

use serde::{Deserialize, Serialize};
use spicex::{ConfigValue, Spice};
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct DatabaseConfig {
    host: String,
    port: u16,
    #[serde(default)]
    ssl: bool,
    #[serde(default = "default_timeout")]
    timeout: u32,
    credentials: Option<Credentials>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct Credentials {
    username: String,
    password: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct ServerConfig {
    name: String,
    port: u16,
    #[serde(default)]
    enabled: bool,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct AppConfig {
    name: String,
    version: String,
    #[serde(default)]
    debug: bool,
    database: DatabaseConfig,
    servers: Vec<ServerConfig>,
    #[serde(default)]
    features: HashMap<String, bool>,
}

fn default_timeout() -> u32 {
    30
}

#[test]
fn test_complete_struct_deserialization() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let config_content = r#"{
        "name": "test-app",
        "version": "1.0.0",
        "debug": true,
        "database": {
            "host": "localhost",
            "port": 5432,
            "ssl": true,
            "credentials": {
                "username": "admin",
                "password": "secret"
            }
        },
        "servers": [
            {
                "name": "web1",
                "port": 8080,
                "enabled": true
            },
            {
                "name": "web2",
                "port": 8081,
                "enabled": false
            }
        ],
        "features": {
            "auth": true,
            "logging": false,
            "metrics": true
        }
    }"#;

    let config_path = temp_dir.path().join("app_config.json");
    fs::write(&config_path, config_content).expect("Failed to write config file");

    let mut spice_instance = Spice::new();
    spice_instance.set_config_file(&config_path).unwrap();

    // Test complete deserialization
    let app_config: AppConfig = spice_instance.unmarshal().unwrap();

    assert_eq!(app_config.name, "test-app");
    assert_eq!(app_config.version, "1.0.0");
    assert_eq!(app_config.debug, true);

    // Test database config
    assert_eq!(app_config.database.host, "localhost");
    assert_eq!(app_config.database.port, 5432);
    assert_eq!(app_config.database.ssl, true);
    assert_eq!(app_config.database.timeout, 30); // default value

    // Test credentials
    let credentials = app_config.database.credentials.unwrap();
    assert_eq!(credentials.username, "admin");
    assert_eq!(credentials.password, "secret");

    // Test servers array
    assert_eq!(app_config.servers.len(), 2);
    assert_eq!(app_config.servers[0].name, "web1");
    assert_eq!(app_config.servers[0].port, 8080);
    assert_eq!(app_config.servers[0].enabled, true);
    assert_eq!(app_config.servers[1].name, "web2");
    assert_eq!(app_config.servers[1].enabled, false);

    // Test features map
    assert_eq!(app_config.features.get("auth"), Some(&true));
    assert_eq!(app_config.features.get("logging"), Some(&false));
    assert_eq!(app_config.features.get("metrics"), Some(&true));
}

#[test]
fn test_partial_struct_deserialization() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let config_content = r#"{
        "name": "partial-app",
        "version": "2.0.0",
        "database": {
            "host": "db-server",
            "port": 3306
        },
        "servers": [],
        "extra_field": "should_be_ignored"
    }"#;

    let config_path = temp_dir.path().join("partial_config.json");
    fs::write(&config_path, config_content).expect("Failed to write config file");

    let mut spice_instance = Spice::new();
    spice_instance.set_config_file(&config_path).unwrap();

    // Test deserialization with missing and default fields
    let app_config: AppConfig = spice_instance.unmarshal().unwrap();

    assert_eq!(app_config.name, "partial-app");
    assert_eq!(app_config.version, "2.0.0");
    assert_eq!(app_config.debug, false); // default value

    // Test database with defaults
    assert_eq!(app_config.database.host, "db-server");
    assert_eq!(app_config.database.port, 3306);
    assert_eq!(app_config.database.ssl, false); // default value
    assert_eq!(app_config.database.timeout, 30); // default function
    assert!(app_config.database.credentials.is_none()); // optional field

    // Test empty servers array
    assert_eq!(app_config.servers.len(), 0);

    // Test empty features map (default)
    assert_eq!(app_config.features.len(), 0);
}

#[test]
fn test_struct_deserialization_with_multiple_sources() {
    // Clean up any existing environment variables
    std::env::remove_var("MYAPP_DATABASE_HOST");
    std::env::remove_var("MYAPP_DATABASE_SSL");
    std::env::remove_var("MYAPP_DEBUG");

    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Base config file
    let base_config = r#"{
        "name": "multi-source-app",
        "version": "1.0.0",
        "database": {
            "host": "localhost",
            "port": 5432
        },
        "servers": [
            {
                "name": "web1",
                "port": 8080
            }
        ]
    }"#;

    let base_path = temp_dir.path().join("base.json");
    fs::write(&base_path, base_config).expect("Failed to write base config");

    let mut spice_instance = Spice::new();

    // Set defaults
    spice_instance
        .set_default("debug", ConfigValue::from(false))
        .unwrap();
    spice_instance
        .set_default("database.ssl", ConfigValue::from(true))
        .unwrap();
    spice_instance
        .set_default("database.timeout", ConfigValue::from(60i64))
        .unwrap();

    // Load base config
    spice_instance.set_config_file(&base_path).unwrap();

    // Set explicit overrides
    spice_instance
        .set("database.host", ConfigValue::from("prod-db"))
        .unwrap();
    spice_instance
        .set("debug", ConfigValue::from(true))
        .unwrap();

    // Add features through explicit setting
    let mut features = HashMap::new();
    features.insert("auth".to_string(), ConfigValue::from(true));
    features.insert("logging".to_string(), ConfigValue::from(false));
    spice_instance
        .set("features", ConfigValue::Object(features))
        .unwrap();

    // Test deserialization with merged sources
    let app_config: AppConfig = spice_instance.unmarshal().unwrap();

    assert_eq!(app_config.name, "multi-source-app"); // from file
    assert_eq!(app_config.version, "1.0.0"); // from file
    assert_eq!(app_config.debug, true); // explicit override

    // Database config from multiple sources
    assert_eq!(app_config.database.host, "prod-db"); // explicit override (highest precedence)
    assert_eq!(app_config.database.port, 5432); // from file
    assert_eq!(app_config.database.ssl, true); // from default
    assert_eq!(app_config.database.timeout, 60); // from default

    // Servers from file
    assert_eq!(app_config.servers.len(), 1);
    assert_eq!(app_config.servers[0].name, "web1");
    assert_eq!(app_config.servers[0].port, 8080);
    assert_eq!(app_config.servers[0].enabled, false); // default

    // Features from explicit setting
    assert_eq!(app_config.features.get("auth"), Some(&true));
    assert_eq!(app_config.features.get("logging"), Some(&false));
}

#[test]
fn test_unmarshal_key_specific_section() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let config_content = r#"{
        "app": {
            "name": "section-test",
            "version": "1.0.0"
        },
        "database": {
            "host": "db-host",
            "port": 5432,
            "ssl": false,
            "credentials": {
                "username": "dbuser",
                "password": "dbpass"
            }
        },
        "other_section": {
            "value": "ignored"
        }
    }"#;

    let config_path = temp_dir.path().join("section_config.json");
    fs::write(&config_path, config_content).expect("Failed to write config file");

    let mut spice_instance = Spice::new();
    spice_instance.set_config_file(&config_path).unwrap();

    // Test unmarshaling specific section
    let db_config: DatabaseConfig = spice_instance.unmarshal_key("database").unwrap();

    assert_eq!(db_config.host, "db-host");
    assert_eq!(db_config.port, 5432);
    assert_eq!(db_config.ssl, false);
    assert_eq!(db_config.timeout, 30); // default value

    let credentials = db_config.credentials.unwrap();
    assert_eq!(credentials.username, "dbuser");
    assert_eq!(credentials.password, "dbpass");
}

#[test]
fn test_struct_deserialization_with_environment_overrides() {
    use std::env;

    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let config_content = r#"{
        "name": "env-test-app",
        "version": "1.0.0",
        "database": {
            "host": "localhost",
            "port": 5432,
            "ssl": false
        },
        "servers": [
            {
                "name": "web1",
                "port": 8080,
                "enabled": true
            }
        ]
    }"#;

    let config_path = temp_dir.path().join("env_config.json");
    fs::write(&config_path, config_content).expect("Failed to write config file");

    // Clean up any existing environment variables first
    env::remove_var("MYAPP_DATABASE_HOST");
    env::remove_var("MYAPP_DATABASE_SSL");
    env::remove_var("MYAPP_DEBUG");

    // Set environment variables
    env::set_var("MYAPP_DATABASE_HOST", "env-db-host");
    env::set_var("MYAPP_DATABASE_SSL", "true");
    env::set_var("MYAPP_DEBUG", "true");

    let mut spice_instance = Spice::new();
    spice_instance.set_config_file(&config_path).unwrap();

    // Add environment layer
    let env_layer = spicex::env_layer::EnvConfigLayer::new(Some("MYAPP".to_string()), true);
    spice_instance.add_layer(Box::new(env_layer));

    // Test deserialization with environment overrides
    let app_config: AppConfig = spice_instance.unmarshal().unwrap();

    assert_eq!(app_config.name, "env-test-app"); // from file
    assert_eq!(app_config.debug, true); // from environment

    // Database config with environment overrides
    assert_eq!(app_config.database.host, "env-db-host"); // from environment
    assert_eq!(app_config.database.port, 5432); // from file
    assert_eq!(app_config.database.ssl, true); // from environment

    // Clean up environment variables
    env::remove_var("MYAPP_DATABASE_HOST");
    env::remove_var("MYAPP_DATABASE_SSL");
    env::remove_var("MYAPP_DEBUG");
}

#[derive(Debug, Deserialize, PartialEq)]
struct NestedArrayConfig {
    groups: Vec<Group>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Group {
    name: String,
    members: Vec<Member>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Member {
    id: u32,
    name: String,
    #[serde(default)]
    active: bool,
}

#[test]
fn test_complex_nested_array_deserialization() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let config_content = r#"{
        "groups": [
            {
                "name": "admins",
                "members": [
                    {
                        "id": 1,
                        "name": "Alice",
                        "active": true
                    },
                    {
                        "id": 2,
                        "name": "Bob"
                    }
                ]
            },
            {
                "name": "users",
                "members": [
                    {
                        "id": 3,
                        "name": "Charlie",
                        "active": false
                    }
                ]
            }
        ]
    }"#;

    let config_path = temp_dir.path().join("nested_config.json");
    fs::write(&config_path, config_content).expect("Failed to write config file");

    let mut spice_instance = Spice::new();
    spice_instance.set_config_file(&config_path).unwrap();

    // Test complex nested array deserialization
    let config: NestedArrayConfig = spice_instance.unmarshal().unwrap();

    assert_eq!(config.groups.len(), 2);

    // Test first group
    let admin_group = &config.groups[0];
    assert_eq!(admin_group.name, "admins");
    assert_eq!(admin_group.members.len(), 2);
    assert_eq!(admin_group.members[0].id, 1);
    assert_eq!(admin_group.members[0].name, "Alice");
    assert_eq!(admin_group.members[0].active, true);
    assert_eq!(admin_group.members[1].id, 2);
    assert_eq!(admin_group.members[1].name, "Bob");
    assert_eq!(admin_group.members[1].active, false); // default value

    // Test second group
    let user_group = &config.groups[1];
    assert_eq!(user_group.name, "users");
    assert_eq!(user_group.members.len(), 1);
    assert_eq!(user_group.members[0].id, 3);
    assert_eq!(user_group.members[0].name, "Charlie");
    assert_eq!(user_group.members[0].active, false);
}
