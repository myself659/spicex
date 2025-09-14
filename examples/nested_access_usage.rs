//! Example demonstrating nested key access and sub-configuration functionality.

use spicex::{ConfigValue, Spice};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== SPICE: Nested Key Access Example ===\n");

    let mut spice_instance = Spice::new();

    // Create a complex nested configuration structure
    let mut server_config = HashMap::new();
    server_config.insert("host".to_string(), ConfigValue::from("localhost"));
    server_config.insert("port".to_string(), ConfigValue::from(8080i64));
    server_config.insert("ssl_enabled".to_string(), ConfigValue::from(true));

    let mut database_config = HashMap::new();
    database_config.insert("host".to_string(), ConfigValue::from("db.example.com"));
    database_config.insert("port".to_string(), ConfigValue::from(5432i64));
    database_config.insert("username".to_string(), ConfigValue::from("admin"));
    database_config.insert("password".to_string(), ConfigValue::from("secret123"));

    let mut app_config = HashMap::new();
    app_config.insert("name".to_string(), ConfigValue::from("MyApp"));
    app_config.insert("version".to_string(), ConfigValue::from("1.0.0"));
    app_config.insert("server".to_string(), ConfigValue::Object(server_config));
    app_config.insert("database".to_string(), ConfigValue::Object(database_config));

    // Create an array of server endpoints
    let endpoints = vec![
        ConfigValue::from("api.example.com"),
        ConfigValue::from("api2.example.com"),
        ConfigValue::from("api3.example.com"),
    ];
    app_config.insert("endpoints".to_string(), ConfigValue::Array(endpoints));

    // Set the complete configuration
    spice_instance.set("app", ConfigValue::Object(app_config))?;

    println!("1. Basic nested key access:");
    println!("   app.name = {:?}", spice_instance.get_string("app.name")?);
    println!(
        "   app.version = {:?}",
        spice_instance.get_string("app.version")?
    );
    println!();

    println!("2. Deep nested key access:");
    println!(
        "   app.server.host = {:?}",
        spice_instance.get_string("app.server.host")?
    );
    println!(
        "   app.server.port = {:?}",
        spice_instance.get_int("app.server.port")?
    );
    println!(
        "   app.server.ssl_enabled = {:?}",
        spice_instance.get_bool("app.server.ssl_enabled")?
    );
    println!();

    println!(
        "   app.database.host = {:?}",
        spice_instance.get_string("app.database.host")?
    );
    println!(
        "   app.database.port = {:?}",
        spice_instance.get_int("app.database.port")?
    );
    println!(
        "   app.database.username = {:?}",
        spice_instance.get_string("app.database.username")?
    );
    println!();

    println!("3. Array index access:");
    println!(
        "   app.endpoints.0 = {:?}",
        spice_instance.get_string("app.endpoints.0")?
    );
    println!(
        "   app.endpoints.1 = {:?}",
        spice_instance.get_string("app.endpoints.1")?
    );
    println!(
        "   app.endpoints.2 = {:?}",
        spice_instance.get_string("app.endpoints.2")?
    );
    println!();

    println!("4. Sub-configuration functionality:");

    // Create a sub-configuration for the server settings
    if let Some(mut server_viper) = spice_instance.sub("app.server")? {
        println!("   Server sub-configuration:");
        println!("     host = {:?}", server_viper.get_string("host")?);
        println!("     port = {:?}", server_viper.get_int("port")?);
        println!(
            "     ssl_enabled = {:?}",
            server_viper.get_bool("ssl_enabled")?
        );
        println!();
    }

    // Create a sub-configuration for the database settings
    if let Some(mut db_viper) = spice_instance.sub("app.database")? {
        println!("   Database sub-configuration:");
        println!("     host = {:?}", db_viper.get_string("host")?);
        println!("     port = {:?}", db_viper.get_int("port")?);
        println!("     username = {:?}", db_viper.get_string("username")?);
        println!("     password = {:?}", db_viper.get_string("password")?);
        println!();
    }

    println!("5. Nested sub-configuration:");

    // Create a sub-configuration for the entire app, then create a nested sub-configuration
    if let Some(app_viper) = spice_instance.sub("app")? {
        if let Some(mut server_viper) = app_viper.sub("server")? {
            println!("   Nested server sub-configuration:");
            println!("     host = {:?}", server_viper.get_string("host")?);
            println!("     port = {:?}", server_viper.get_int("port")?);
            println!();
        }
    }

    println!("6. Custom key delimiter:");
    let mut custom_viper = Spice::new();
    custom_viper.set_key_delimiter("::");

    let mut config = HashMap::new();
    config.insert("host".to_string(), ConfigValue::from("custom.example.com"));
    custom_viper.set("database", ConfigValue::Object(config))?;

    println!("   Using '::' as delimiter:");
    println!(
        "   database::host = {:?}",
        custom_viper.get_string("database::host")?
    );
    println!();

    println!("7. Error handling for non-existent keys:");
    println!(
        "   app.nonexistent = {:?}",
        spice_instance.get_string("app.nonexistent")?
    );
    println!(
        "   app.endpoints.10 = {:?}",
        spice_instance.get_string("app.endpoints.10")?
    );
    println!();

    println!("8. Exact key priority over nested access:");
    // Set an exact key that would conflict with nested access
    spice_instance.set("app.server.host", ConfigValue::from("exact_key_value"))?;
    println!("   After setting exact key 'app.server.host':");
    println!(
        "   app.server.host = {:?}",
        spice_instance.get_string("app.server.host")?
    );
    println!("   (exact key takes precedence over nested access)");

    println!("\n=== Example completed successfully! ===");
    Ok(())
}
