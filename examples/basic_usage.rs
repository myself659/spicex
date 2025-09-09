//! Basic usage example for SPICE configuration library.

use spicex::{ConfigValue, Spice};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("SPICE - Basic Usage Example");
    println!("================================");

    // Create a new Spice instance
    let mut spice_instance = Spice::new();

    println!("✓ Created new Spice instance");

    // This example demonstrates the basic structure
    // Full functionality will be implemented in subsequent tasks

    println!("📝 Note: This is a basic structure example.");
    println!("   Full configuration loading will be available after implementing the core functionality.");

    Ok(())
}
