# SpiceX

A complete configuration solution for Rust applications, inspired by [viper](https://github.com/spf13/viper).

SpiceX is designed to work within an application and can handle all types of configuration needs and formats. It provides a unified interface for reading configuration from multiple sources with a clear precedence hierarchy.

## Features

- ✅ **Multiple Configuration Sources** - Files, environment variables, command line flags, defaults
- ✅ **Multiple File Formats** - JSON, YAML, TOML, INI support
- ✅ **Precedence Hierarchy** - Clear ordering of configuration sources
- ✅ **Nested Configuration** - Dot notation access to nested values
- ✅ **Type Safety** - Strong typing with automatic type conversion
- ✅ **Struct Deserialization** - Deserialize configuration into Rust structs
- ✅ **File Watching** - Automatic reloading when configuration files change
- ✅ **Environment Variables** - Automatic mapping with prefix support
- ✅ **Command Line Flags** - Integration with clap for CLI arguments
- ✅ **Default Values** - Fallback values for missing configuration
- ✅ **Configuration Writing** - Save configuration back to files

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
spicex = "0.1.0"

# Optional: Enable CLI support
[dependencies.spicex]
version = "0.1.0"
features = ["cli"]
```

### Basic Usage

```rust
use spicex::{Spice, ConfigValue};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut spice = Spice::new();

    // Set default values
    spice.set_default("database.host", ConfigValue::from("localhost"))?;
    spice.set_default("database.port", ConfigValue::from(5432i64))?;
    spice.set_default("debug", ConfigValue::from(false))?;

    // Configure file discovery
    spice.set_config_name("config");
    spice.add_config_path(".");
    spice.add_config_path("./configs");
    spice.add_config_path("/etc/myapp");

    // Try to read configuration file
    if let Err(e) = spice.read_in_config() {
        println!("No config file found, using defaults: {}", e);
    }

    // Set up environment variable support
    spice.set_env_prefix("MYAPP");
    spice.set_automatic_env(true);

    // Access configuration values
    let host = spice.get_string("database.host")?.unwrap_or_default();
    let port = spice.get_i64("database.port")?.unwrap_or(5432);
    let debug = spice.get_bool("debug")?.unwrap_or(false);

    println!("Database: {}:{}", host, port);
    println!("Debug mode: {}", debug);

    Ok(())
}
```

### With Struct Deserialization

```rust
use spicex::{Spice, ConfigValue};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct DatabaseConfig {
    host: String,
    port: u16,
    #[serde(default)]
    ssl: bool,
}

#[derive(Deserialize, Debug)]
struct AppConfig {
    database: DatabaseConfig,
    debug: bool,
    #[serde(default)]
    log_level: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut spice = Spice::new();

    // Set defaults
    spice.set_default("database.host", ConfigValue::from("localhost"))?;
    spice.set_default("database.port", ConfigValue::from(5432i64))?;
    spice.set_default("debug", ConfigValue::from(false))?;
    spice.set_default("log_level", ConfigValue::from("info"))?;

    // Load configuration
    spice.set_config_name("config");
    spice.add_config_path(".");
    let _ = spice.read_in_config(); // Ignore errors, use defaults

    // Deserialize into struct
    let config: AppConfig = spice.unmarshal()?;
    println!("Configuration: {:#?}", config);

    Ok(())
}
```

## Configuration Precedence

Spice uses the following precedence order (highest to lowest):

1. **Explicit calls** - Values set via `spice.set()`
2. **Command line flags** - CLI arguments (requires `cli` feature)
3. **Environment variables** - System environment variables
4. **Configuration files** - JSON, YAML, TOML, INI files
5. **Key/value stores** - Remote configuration (future feature)
6. **Default values** - Fallback values set via `spice.set_default()`

## Configuration File Formats

### JSON Example (`config.json`)

```json
{
  "database": {
    "host": "localhost",
    "port": 5432,
    "ssl": true,
    "credentials": {
      "username": "admin",
      "password": "secret"
    }
  },
  "server": {
    "port": 8080,
    "host": "0.0.0.0"
  },
  "features": ["auth", "logging", "metrics"],
  "debug": false
}
```

### YAML Example (`config.yaml`)

```yaml
database:
  host: localhost
  port: 5432
  ssl: true
  credentials:
    username: admin
    password: secret

server:
  port: 8080
  host: 0.0.0.0

features:
  - auth
  - logging
  - metrics

debug: false
```

### TOML Example (`config.toml`)

```toml
debug = false
features = ["auth", "logging", "metrics"]

[database]
host = "localhost"
port = 5432
ssl = true

[database.credentials]
username = "admin"
password = "secret"

[server]
port = 8080
host = "0.0.0.0"
```

### INI Example (`config.ini`)

```ini
debug = false

[database]
host = localhost
port = 5432
ssl = true

[server]
port = 8080
host = 0.0.0.0
```

## Environment Variables

Environment variables are automatically mapped to configuration keys:

```bash
# Set environment variables
export MYAPP_DATABASE_HOST=production-db
export MYAPP_DATABASE_PORT=5432
export MYAPP_DEBUG=true

# These become available as:
# database.host = "production-db"
# database.port = 5432
# debug = true
```

```rust
use spicex::Spice;

let mut spice = Spice::new();
spice.set_env_prefix("MYAPP");
spice.set_automatic_env(true);

// Access environment variables
let host = spice.get_string("database.host")?;
let debug = spice.get_bool("debug")?;
```

## Command Line Flags

With the `cli` feature enabled, you can integrate with clap:

```rust
use spicex::Spice;
use clap::{Arg, Command};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Command::new("myapp")
        .arg(Arg::new("host")
            .long("host")
            .value_name("HOST")
            .help("Database host"))
        .arg(Arg::new("port")
            .long("port")
            .value_name("PORT")
            .help("Database port"))
        .arg(Arg::new("debug")
            .long("debug")
            .action(clap::ArgAction::SetTrue)
            .help("Enable debug mode"));

    let matches = app.get_matches();

    let mut spice = Spice::new();
    spice.bind_flags(matches);

    // CLI flags now override other configuration sources
    let host = spice.get_string("host")?;
    let debug = spice.get_bool("debug")?;

    Ok(())
}
```

## File Watching

Enable automatic reloading when configuration files change:

```rust
use spicex::Spice;
use std::sync::{Arc, Mutex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut spice = Spice::new();
    spice.set_config_name("config");
    spice.read_in_config()?;

    // Enable file watching
    spice.watch_config()?;

    // Register callback for configuration changes
    let reload_count = Arc::new(Mutex::new(0));
    let count_clone = Arc::clone(&reload_count);

    spice.on_config_change(move || {
        let mut count = count_clone.lock().unwrap();
        *count += 1;
        println!("Configuration reloaded {} times", *count);
    })?;

    // Your application continues running...
    // Configuration will automatically reload when files change

    Ok(())
}
```

## Writing Configuration

Save current configuration to files:

```rust
use spicex::{Spice, ConfigValue};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut spice = Spice::new();

    // Set some configuration
    spice.set("app.name", ConfigValue::from("My Application"))?;
    spice.set("app.version", ConfigValue::from("1.0.0"))?;
    spice.set("database.host", ConfigValue::from("localhost"))?;

    // Write to different formats
    spice.write_config("output.json")?;        // JSON format
    spice.write_config("output.yaml")?;        // YAML format
    spice.write_config_as("output.txt", "toml")?; // TOML in .txt file

    // Safe write (won't overwrite existing files)
    spice.safe_write_config("backup.json")?;

    Ok(())
}
```

## Advanced Usage

### Sub-configurations

Work with configuration subsections:

```rust
use spicex::{Spice, ConfigValue};
use std::collections::HashMap;

let mut spice = Spice::new();

// Set up nested configuration
let mut db_config = HashMap::new();
db_config.insert("host".to_string(), ConfigValue::from("localhost"));
db_config.insert("port".to_string(), ConfigValue::from(5432i64));
spice.set("database", ConfigValue::Object(db_config))?;

// Create sub-configuration for database settings
if let Some(db_viper) = spice.sub("database")? {
    // Access "host" directly instead of "database.host"
    let host = db_viper.get_string("host")?;
    let port = db_viper.get_i64("port")?;
}
```

### Configuration Validation

Validate configuration during deserialization:

```rust
use spicex::{Spice, ConfigValue, ConfigError};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct ServerConfig {
    host: String,
    port: u16,
}

impl ServerConfig {
    fn validate(&self) -> Result<(), String> {
        if self.port < 1024 {
            return Err("Port must be >= 1024".to_string());
        }
        if self.host.is_empty() {
            return Err("Host cannot be empty".to_string());
        }
        Ok(())
    }
}

let mut spice = Spice::new();
spice.set("host", ConfigValue::from("localhost"))?;
spice.set("port", ConfigValue::from(8080i64))?;

let config: ServerConfig = spice.unmarshal_with_validation(|config: &ServerConfig| {
    config.validate().map_err(|e| ConfigError::invalid_value(e))
})?;
```

## Error Handling

Spice provides detailed error information:

```rust
use spicex::{Spice, ConfigError};

let spice = Spice::new();

match spice.get_string("nonexistent.key") {
    Ok(Some(value)) => println!("Value: {}", value),
    Ok(None) => println!("Key not found"),
    Err(ConfigError::KeyNotFound { key }) => {
        println!("Key '{}' not found", key);
    }
    Err(ConfigError::TypeConversion { from, to }) => {
        println!("Cannot convert {} to {}", from, to);
    }
    Err(ConfigError::Parse { source_name, message }) => {
        println!("Parse error in {}: {}", source_name, message);
    }
    Err(e) => println!("Other error: {}", e),
}
```

## Migration from Other Libraries

### From `config` crate

```rust
// Old way (config crate)
use config::{Config, ConfigError, File};

let settings = Config::builder()
    .add_source(File::with_name("config"))
    .build()?;
let host: String = settings.get("database.host")?;

// New way (spice)
use spicex::Spice;

let mut spice = Spice::new();
spice.set_config_name("config");
spice.read_in_config()?;
let host = spice.get_string("database.host")?.unwrap_or_default();
```

### From Environment Variables Only

```rust
// Old way (std::env)
use std::env;

let host = env::var("DATABASE_HOST").unwrap_or_else(|_| "localhost".to_string());
let port: u16 = env::var("DATABASE_PORT")
    .unwrap_or_else(|_| "5432".to_string())
    .parse()
    .unwrap_or(5432);

// New way (spice)
use spicex::{Spice, ConfigValue};

let mut spice = Spice::new();
spice.set_default("database.host", ConfigValue::from("localhost"))?;
spice.set_default("database.port", ConfigValue::from(5432i64))?;
spice.set_env_prefix("DATABASE");
spice.set_automatic_env(true);

let host = spice.get_string("host")?.unwrap_or_default();
let port = spice.get_i64("port")?.unwrap_or(5432) as u16;
```

## Examples

The `examples/` directory contains comprehensive examples:

- [`basic_usage.rs`](examples/basic_usage.rs) - Basic configuration loading and access
- [`struct_deserialization.rs`](examples/struct_deserialization.rs) - Deserializing into structs
- [`env_layer_usage.rs`](examples/env_layer_usage.rs) - Environment variable configuration
- [`file_watching.rs`](examples/file_watching.rs) - Watching for configuration changes
- [`cli_flag_usage.rs`](examples/cli_flag_usage.rs) - Command line flag integration
- [`nested_access_usage.rs`](examples/nested_access_usage.rs) - Working with nested configuration
- [`default_values_usage.rs`](examples/default_values_usage.rs) - Setting and using defaults
- [`file_discovery_usage.rs`](examples/file_discovery_usage.rs) - Automatic file discovery
- [`web_server_config.rs`](examples/web_server_config.rs) - Real-world web server configuration
- [`microservice_config.rs`](examples/microservice_config.rs) - Microservice configuration patterns

## Performance

Spice-rust is designed for performance:

- **Lazy Loading** - Configuration sources are loaded on-demand
- **Caching** - Values are cached after first access
- **Zero-Copy** - Minimal allocations through strategic use of references
- **Efficient Parsing** - Uses optimized parsers for each format

Run benchmarks with:

```bash
cargo run --example performance_benchmarks --release
```

## Contributing

Contributions are welcome! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Setup

```bash
git clone https://github.com/myself659/spicex.git
cd spicex
cargo build
cargo test
```

### Running Examples

```bash
# Basic usage
cargo run --example basic_usage

# With CLI support
cargo run --example cli_flag_usage --features cli -- --host localhost --port 8080

# File watching (requires a config file)
echo '{"debug": true}' > config.json
cargo run --example file_watching
```

## License


- MIT license ([LICENSE-MIT](LICENSE-MIT))


at your option.

## Acknowledgments

- Inspired by [viper](https://github.com/spf13/viper) for Go
- Built with the excellent Rust ecosystem including serde, clap, notify, and more