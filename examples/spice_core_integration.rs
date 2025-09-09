//! Integration test for Spice  core functionality

use spicex::{ConfigValue, EnvConfigLayer, Spice};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create Spice instance
    let mut spice_instance = Spice::new();

    // Add environment layer
    let env_layer = EnvConfigLayer::new(Some("MYAPP".to_string()), false);
    spice_instance.add_layer(Box::new(env_layer));

    // Set some explicit values
    spice_instance.set("app.name", ConfigValue::from("MyApp"))?;
    spice_instance.set("app.version", ConfigValue::from("1.0.0"))?;
    spice_instance.set("database.port", ConfigValue::from(5432i64))?;
    spice_instance.set("debug", ConfigValue::from(true))?;

    // Test getting values with precedence
    println!("App name: {:?}", spice_instance.get_string("app.name")?);
    println!(
        "App version: {:?}",
        spice_instance.get_string("app.version")?
    );
    println!(
        "Database port: {:?}",
        spice_instance.get_int("database.port")?
    );
    println!("Debug mode: {:?}", spice_instance.get_bool("debug")?);

    // Test layer info
    println!("Layers: {:?}", spice_instance.layer_info());
    println!("All keys: {:?}", spice_instance.all_keys());

    // Test type conversions
    println!(
        "Port as string: {:?}",
        spice_instance.get_string("database.port")?
    );
    println!("Debug as string: {:?}", spice_instance.get_string("debug")?);

    println!("Integration test completed successfully!");
    Ok(())
}
