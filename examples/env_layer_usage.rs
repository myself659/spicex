//! Example demonstrating environment variable configuration layer usage.

use spicex::{ConfigLayer, ConfigValue, EnvConfigLayer};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Environment Variable Configuration Layer Example");
    println!("================================================");

    // Set some test environment variables
    env::set_var("APP_DATABASE_HOST", "localhost");
    env::set_var("APP_DATABASE_PORT", "5432");
    env::set_var("APP_DEBUG", "true");
    env::set_var("APP_MAX_CONNECTIONS", "100");
    env::set_var("APP_TIMEOUT", "30.5");

    // Create environment layer with prefix
    let env_layer = EnvConfigLayer::new(Some("APP".to_string()), true);

    println!("\n1. Basic environment variable access:");

    // Test basic access
    if let Some(host) = env_layer.get("database.host")? {
        println!("   database.host = {:?}", host);
    }

    if let Some(port) = env_layer.get("database.port")? {
        println!("   database.port = {:?}", port);
    }

    if let Some(debug) = env_layer.get("debug")? {
        println!("   debug = {:?}", debug);
    }

    if let Some(max_conn) = env_layer.get("max.connections")? {
        println!("   max.connections = {:?}", max_conn);
    }

    if let Some(timeout) = env_layer.get("timeout")? {
        println!("   timeout = {:?}", timeout);
    }

    println!("\n2. Key transformation demonstration:");
    println!(
        "   Config key 'database.host' -> Env var '{}'",
        env_layer.transform_key("database.host")
    );
    println!(
        "   Config key 'nested.config.value' -> Env var '{}'",
        env_layer.transform_key("nested.config.value")
    );

    println!("\n3. Available keys in automatic mode:");
    let keys = env_layer.keys();
    for key in &keys {
        if key.starts_with("database")
            || key.starts_with("debug")
            || key.starts_with("max")
            || key.starts_with("timeout")
        {
            println!("   - {}", key);
        }
    }

    println!("\n4. Custom key replacer example:");
    let mut custom_env_layer = EnvConfigLayer::new(Some("CUSTOM".to_string()), false);
    custom_env_layer.set_key_replacer(Box::new(|key: &str| key.replace("_", "__")));

    println!(
        "   With custom replacer: 'app.config' -> '{}'",
        custom_env_layer.transform_key("app.config")
    );

    // Clean up test environment variables
    env::remove_var("APP_DATABASE_HOST");
    env::remove_var("APP_DATABASE_PORT");
    env::remove_var("APP_DEBUG");
    env::remove_var("APP_MAX_CONNECTIONS");
    env::remove_var("APP_TIMEOUT");

    println!("\n✅ Environment variable layer example completed successfully!");
    Ok(())
}
