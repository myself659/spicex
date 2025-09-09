//! Performance benchmarks for SPICE
//!
//! This example demonstrates performance characteristics of the configuration
//! system and provides benchmarks for key operations.

use spicex::{ConfigValue, EnvConfigLayer, Spice};
use std::collections::HashMap;
use std::fs;
use std::time::{Duration, Instant};
use tempfile::TempDir;

struct BenchmarkResult {
    operation: String,
    iterations: usize,
    total_time: Duration,
    avg_time: Duration,
    ops_per_second: f64,
}

impl BenchmarkResult {
    fn new(operation: String, iterations: usize, total_time: Duration) -> Self {
        let avg_time = total_time / iterations as u32;
        let ops_per_second = iterations as f64 / total_time.as_secs_f64();

        Self {
            operation,
            iterations,
            total_time,
            avg_time,
            ops_per_second,
        }
    }

    fn display(&self) {
        println!(
            "  {:<30} | {:>8} ops | {:>10.2?} | {:>12.2?} | {:>12.0} ops/s",
            self.operation, self.iterations, self.total_time, self.avg_time, self.ops_per_second
        );
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Spice -Rust Performance Benchmarks");
    println!("====================================");

    let mut results = Vec::new();

    // Setup test data
    let temp_dir = setup_test_environment()?;

    // Benchmark configuration loading
    results.push(benchmark_config_loading(&temp_dir)?);

    // Benchmark key access patterns
    results.extend(benchmark_key_access(&temp_dir)?);

    // Benchmark type conversions
    results.extend(benchmark_type_conversions(&temp_dir)?);

    // Benchmark layer operations
    results.extend(benchmark_layer_operations(&temp_dir)?);

    // Benchmark environment variable access
    results.push(benchmark_environment_access()?);

    // Benchmark struct deserialization
    results.push(benchmark_struct_deserialization(&temp_dir)?);

    // Display results
    display_benchmark_results(&results);

    // Memory usage analysis
    analyze_memory_usage(&temp_dir)?;

    println!("\n✅ Performance benchmarks completed!");
    Ok(())
}

fn setup_test_environment() -> Result<TempDir, Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    // Create a large configuration file for testing
    let large_config = create_large_config();
    fs::write(temp_dir.path().join("large_config.json"), large_config)?;

    // Create a nested configuration file
    let nested_config = create_nested_config();
    fs::write(temp_dir.path().join("nested_config.json"), nested_config)?;

    // Create a simple configuration file
    let simple_config = r#"{
        "app": {
            "name": "benchmark-app",
            "version": "1.0.0",
            "debug": false
        },
        "database": {
            "host": "localhost",
            "port": 5432,
            "ssl": true
        }
    }"#;
    fs::write(temp_dir.path().join("simple_config.json"), simple_config)?;

    Ok(temp_dir)
}

fn benchmark_config_loading(
    temp_dir: &TempDir,
) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
    let iterations = 1000;
    let config_path = temp_dir.path().join("simple_config.json");

    let start = Instant::now();

    for _ in 0..iterations {
        let mut spice_instance = Spice::new();
        spice_instance.set_config_file(&config_path)?;
        // Force loading by accessing a value
        let _ = spice_instance.get_string("app.name")?;
    }

    let elapsed = start.elapsed();
    Ok(BenchmarkResult::new(
        "Config File Loading".to_string(),
        iterations,
        elapsed,
    ))
}

fn benchmark_key_access(
    temp_dir: &TempDir,
) -> Result<Vec<BenchmarkResult>, Box<dyn std::error::Error>> {
    let mut results = Vec::new();
    let config_path = temp_dir.path().join("nested_config.json");

    let mut spice_instance = Spice::new();
    spice_instance.set_config_file(&config_path)?;

    // Benchmark simple key access
    let iterations = 100000;
    let start = Instant::now();

    for _ in 0..iterations {
        let _ = spice_instance.get_string("app.name")?;
    }

    let elapsed = start.elapsed();
    results.push(BenchmarkResult::new(
        "Simple Key Access".to_string(),
        iterations,
        elapsed,
    ));

    // Benchmark nested key access
    let iterations = 50000;
    let start = Instant::now();

    for _ in 0..iterations {
        let _ = spice_instance.get_string("database.connection.pool.max_size")?;
    }

    let elapsed = start.elapsed();
    results.push(BenchmarkResult::new(
        "Nested Key Access".to_string(),
        iterations,
        elapsed,
    ));

    // Benchmark array access
    let iterations = 30000;
    let start = Instant::now();

    for _ in 0..iterations {
        let _ = spice_instance.get_string("servers.0.host")?;
    }

    let elapsed = start.elapsed();
    results.push(BenchmarkResult::new(
        "Array Index Access".to_string(),
        iterations,
        elapsed,
    ));

    Ok(results)
}

fn benchmark_type_conversions(
    temp_dir: &TempDir,
) -> Result<Vec<BenchmarkResult>, Box<dyn std::error::Error>> {
    let mut results = Vec::new();
    let config_path = temp_dir.path().join("nested_config.json");

    let mut spice_instance = Spice::new();
    spice_instance.set_config_file(&config_path)?;

    // Benchmark string access
    let iterations = 100000;
    let start = Instant::now();

    for _ in 0..iterations {
        let _ = spice_instance.get_string("app.name")?;
    }

    let elapsed = start.elapsed();
    results.push(BenchmarkResult::new(
        "String Conversion".to_string(),
        iterations,
        elapsed,
    ));

    // Benchmark integer access
    let iterations = 100000;
    let start = Instant::now();

    for _ in 0..iterations {
        let _ = spice_instance.get_int("database.port")?;
    }

    let elapsed = start.elapsed();
    results.push(BenchmarkResult::new(
        "Integer Conversion".to_string(),
        iterations,
        elapsed,
    ));

    // Benchmark boolean access
    let iterations = 100000;
    let start = Instant::now();

    for _ in 0..iterations {
        let _ = spice_instance.get_bool("app.debug")?;
    }

    let elapsed = start.elapsed();
    results.push(BenchmarkResult::new(
        "Boolean Conversion".to_string(),
        iterations,
        elapsed,
    ));

    // Benchmark float access
    let iterations = 100000;
    let start = Instant::now();

    for _ in 0..iterations {
        let _ = spice_instance.get_float("performance.cpu_threshold")?;
    }

    let elapsed = start.elapsed();
    results.push(BenchmarkResult::new(
        "Float Conversion".to_string(),
        iterations,
        elapsed,
    ));

    Ok(results)
}

fn benchmark_layer_operations(
    temp_dir: &TempDir,
) -> Result<Vec<BenchmarkResult>, Box<dyn std::error::Error>> {
    let mut results = Vec::new();

    // Benchmark adding layers
    let iterations = 10000;
    let start = Instant::now();

    for _ in 0..iterations {
        let mut spice_instance = Spice::new();

        // Add multiple layers
        spice_instance.set_default("key1", ConfigValue::from("default"))?;

        let env_layer = EnvConfigLayer::new(Some("TEST".to_string()), false);
        spice_instance.add_layer(Box::new(env_layer));

        spice_instance.set("key1", ConfigValue::from("explicit"))?;
    }

    let elapsed = start.elapsed();
    results.push(BenchmarkResult::new(
        "Layer Management".to_string(),
        iterations,
        elapsed,
    ));

    // Benchmark precedence resolution with multiple layers
    let mut spice_instance = Spice::new();
    spice_instance.set_default("test_key", ConfigValue::from("default"))?;

    let env_layer = EnvConfigLayer::new(Some("BENCH".to_string()), false);
    spice_instance.add_layer(Box::new(env_layer));

    let config_path = temp_dir.path().join("simple_config.json");
    spice_instance.set_config_file(&config_path)?;

    let iterations = 50000;
    let start = Instant::now();

    for _ in 0..iterations {
        let _ = spice_instance.get_string("app.name")?; // This will go through precedence resolution
    }

    let elapsed = start.elapsed();
    results.push(BenchmarkResult::new(
        "Precedence Resolution".to_string(),
        iterations,
        elapsed,
    ));

    Ok(results)
}

fn benchmark_environment_access() -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
    // Set up test environment variables
    std::env::set_var("BENCH_TEST_VAR1", "value1");
    std::env::set_var("BENCH_TEST_VAR2", "value2");
    std::env::set_var("BENCH_NESTED_CONFIG_VALUE", "nested_value");

    let mut spice_instance = Spice::new();
    let env_layer = EnvConfigLayer::new(Some("BENCH".to_string()), true);
    spice_instance.add_layer(Box::new(env_layer));

    let iterations = 50000;
    let start = Instant::now();

    for _ in 0..iterations {
        let _ = spice_instance.get_string("test.var1")?;
        let _ = spice_instance.get_string("nested.config.value")?;
    }

    let elapsed = start.elapsed();

    // Clean up
    std::env::remove_var("BENCH_TEST_VAR1");
    std::env::remove_var("BENCH_TEST_VAR2");
    std::env::remove_var("BENCH_NESTED_CONFIG_VALUE");

    Ok(BenchmarkResult::new(
        "Environment Variable Access".to_string(),
        iterations,
        elapsed,
    ))
}

fn benchmark_struct_deserialization(
    temp_dir: &TempDir,
) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct BenchmarkConfig {
        app: AppConfig,
        database: DatabaseConfig,
        servers: Vec<ServerConfig>,
    }

    #[derive(Deserialize)]
    struct AppConfig {
        name: String,
        version: String,
        debug: bool,
    }

    #[derive(Deserialize)]
    struct DatabaseConfig {
        host: String,
        port: u16,
        ssl: bool,
    }

    #[derive(Deserialize)]
    struct ServerConfig {
        host: String,
        port: u16,
        enabled: bool,
    }

    let config_path = temp_dir.path().join("nested_config.json");
    let mut spice_instance = Spice::new();
    spice_instance.set_config_file(&config_path)?;

    let iterations = 5000;
    let start = Instant::now();

    for _ in 0..iterations {
        let _config: BenchmarkConfig = spice_instance.unmarshal()?;
    }

    let elapsed = start.elapsed();
    Ok(BenchmarkResult::new(
        "Struct Deserialization".to_string(),
        iterations,
        elapsed,
    ))
}

fn display_benchmark_results(results: &[BenchmarkResult]) {
    println!("\n📊 Benchmark Results:");
    println!("====================");
    println!(
        "  {:<30} | {:>8} | {:>10} | {:>12} | {:>12}",
        "Operation", "Iterations", "Total Time", "Avg Time", "Ops/Second"
    );
    println!(
        "  {:-<30}-+-{:-<8}-+-{:-<10}-+-{:-<12}-+-{:-<12}",
        "", "", "", "", ""
    );

    for result in results {
        result.display();
    }

    // Calculate and display summary statistics
    let total_ops: usize = results.iter().map(|r| r.iterations).sum();
    let total_time: Duration = results.iter().map(|r| r.total_time).sum();
    let avg_ops_per_second: f64 =
        results.iter().map(|r| r.ops_per_second).sum::<f64>() / results.len() as f64;

    println!(
        "  {:-<30}-+-{:-<8}-+-{:-<10}-+-{:-<12}-+-{:-<12}",
        "", "", "", "", ""
    );
    println!(
        "  {:<30} | {:>8} | {:>10.2?} | {:>12} | {:>12.0}",
        "TOTAL", total_ops, total_time, "-", avg_ops_per_second
    );
}

fn analyze_memory_usage(temp_dir: &TempDir) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🧠 Memory Usage Analysis:");
    println!("========================");

    // Create configurations of different sizes
    let configs = vec![
        ("Small Config (10 keys)", create_small_config()),
        ("Medium Config (100 keys)", create_medium_config()),
        ("Large Config (1000 keys)", create_large_config()),
    ];

    for (name, config_content) in configs {
        let config_path = temp_dir
            .path()
            .join(format!("{}.json", name.replace(" ", "_").to_lowercase()));
        fs::write(&config_path, config_content)?;

        let mut spice_instance = Spice::new();
        spice_instance.set_config_file(&config_path)?;

        // Force loading all values
        let _ = spice_instance.all_settings()?;

        println!("  {}: Configuration loaded", name);
        // Note: In a real benchmark, you would measure actual memory usage here
        // This would require platform-specific code or external tools
    }

    println!("  💡 Tip: Use tools like valgrind or heaptrack for detailed memory analysis");

    Ok(())
}

fn create_small_config() -> String {
    r#"{
  "app": {
    "name": "small-app",
    "version": "1.0.0",
    "debug": false
  },
  "database": {
    "host": "localhost",
    "port": 5432
  }
}"#
    .to_string()
}

fn create_medium_config() -> String {
    let mut config = HashMap::new();

    // Add 100 keys with various nesting levels
    for i in 0..100 {
        let section = format!("section_{}", i / 10);
        let key = format!("key_{}", i);
        let value = format!("value_{}", i);

        config
            .entry(section)
            .or_insert_with(HashMap::new)
            .insert(key, value);
    }

    serde_json::to_string_pretty(&config).unwrap_or_default()
}

fn create_large_config() -> String {
    let mut config = HashMap::new();

    // Add 1000 keys with deep nesting
    for i in 0..1000 {
        let section1 = format!("section_{}", i / 100);
        let section2 = format!("subsection_{}", i / 10);
        let key = format!("key_{}", i);
        let value = if i % 3 == 0 {
            serde_json::Value::String(format!("string_value_{}", i))
        } else if i % 3 == 1 {
            serde_json::Value::Number(serde_json::Number::from(i))
        } else {
            serde_json::Value::Bool(i % 2 == 0)
        };

        config
            .entry(section1)
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))
            .as_object_mut()
            .unwrap()
            .entry(section2)
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))
            .as_object_mut()
            .unwrap()
            .insert(key, value);
    }

    serde_json::to_string_pretty(&config).unwrap_or_default()
}

fn create_nested_config() -> String {
    r#"{
  "app": {
    "name": "nested-benchmark-app",
    "version": "2.1.0",
    "debug": true,
    "features": {
      "auth": true,
      "logging": false,
      "metrics": true
    }
  },
  "database": {
    "host": "localhost",
    "port": 5432,
    "ssl": true,
    "connection": {
      "pool": {
        "max_size": 20,
        "min_size": 5,
        "timeout": 30
      },
      "retry": {
        "attempts": 3,
        "delay": 1000
      }
    }
  },
  "servers": [
    {
      "host": "server1.example.com",
      "port": 8080,
      "enabled": true,
      "weight": 100
    },
    {
      "host": "server2.example.com",
      "port": 8081,
      "enabled": true,
      "weight": 50
    },
    {
      "host": "server3.example.com",
      "port": 8082,
      "enabled": false,
      "weight": 25
    }
  ],
  "performance": {
    "cpu_threshold": 0.8,
    "memory_threshold": 0.9,
    "disk_threshold": 0.95
  },
  "monitoring": {
    "metrics": {
      "enabled": true,
      "port": 9090,
      "path": "/metrics",
      "interval": 15
    },
    "health_check": {
      "enabled": true,
      "path": "/health",
      "timeout": 5
    }
  }
}"#
    .to_string()
}
