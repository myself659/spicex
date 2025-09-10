//! Real-world example: CLI application with comprehensive configuration
//!
//! This example demonstrates how to build a CLI application that uses
//! configuration files, environment variables, and command-line arguments
//! with proper precedence and validation.

use clap::{Arg, ArgMatches, Command};
use serde::{Deserialize, Serialize};
use spicex::{ConfigValue, EnvConfigLayer, Spice};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct CliAppConfig {
    #[serde(default = "default_app_name")]
    name: String,
    #[serde(default = "default_version")]
    version: String,
    #[serde(default)]
    verbose: bool,
    #[serde(default)]
    quiet: bool,
    #[serde(default)]
    dry_run: bool,

    #[serde(default)]
    input: InputConfig,
    #[serde(default)]
    output: OutputConfig,
    #[serde(default)]
    processing: ProcessingConfig,
    #[serde(default)]
    logging: LoggingConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct InputConfig {
    #[serde(default)]
    files: Vec<String>,
    #[serde(default = "default_input_format")]
    format: String,
    #[serde(default)]
    recursive: bool,
    #[serde(default)]
    follow_symlinks: bool,
    #[serde(default)]
    include_patterns: Vec<String>,
    #[serde(default)]
    exclude_patterns: Vec<String>,
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            files: Vec::new(),
            format: default_input_format(),
            recursive: false,
            follow_symlinks: false,
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct OutputConfig {
    #[serde(default)]
    file: Option<String>,
    #[serde(default = "default_output_format")]
    format: String,
    #[serde(default)]
    overwrite: bool,
    #[serde(default)]
    create_dirs: bool,
    #[serde(default = "default_compression")]
    compression: String,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            file: None,
            format: default_output_format(),
            overwrite: false,
            create_dirs: true,
            compression: default_compression(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ProcessingConfig {
    #[serde(default = "default_workers")]
    workers: u32,
    #[serde(default = "default_batch_size")]
    batch_size: u32,
    #[serde(default = "default_timeout")]
    timeout: u64,
    #[serde(default)]
    parallel: bool,
    #[serde(default)]
    memory_limit: Option<u64>,
    #[serde(default)]
    options: HashMap<String, String>,
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            workers: default_workers(),
            batch_size: default_batch_size(),
            timeout: default_timeout(),
            parallel: false,
            memory_limit: None,
            options: HashMap::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct LoggingConfig {
    #[serde(default = "default_log_level")]
    level: String,
    #[serde(default)]
    file: Option<String>,
    #[serde(default)]
    timestamp: bool,
    #[serde(default)]
    colors: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            file: None,
            timestamp: true,
            colors: true,
        }
    }
}

// Default value functions
fn default_app_name() -> String {
    "cli-processor".to_string()
}
fn default_version() -> String {
    "1.0.0".to_string()
}
fn default_input_format() -> String {
    "auto".to_string()
}
fn default_output_format() -> String {
    "json".to_string()
}
fn default_compression() -> String {
    "none".to_string()
}
fn default_workers() -> u32 {
    num_cpus::get() as u32
}
fn default_batch_size() -> u32 {
    100
}
fn default_timeout() -> u64 {
    30
}
fn default_log_level() -> String {
    "info".to_string()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 CLI Application Configuration Example");
    println!("========================================");

    // 1. Parse command line arguments
    let matches = build_cli().get_matches();

    // 2. Set up configuration with proper precedence
    let mut spice_instance = setup_configuration(&matches)?;

    // 3. Load and validate the final configuration
    let config: CliAppConfig = spice_instance.unmarshal()?;

    // 4. Display configuration summary
    display_configuration(&config, &matches);

    // 5. Demonstrate configuration usage in application logic
    demonstrate_application_logic(&config)?;

    // 6. Show how to handle configuration changes
    demonstrate_dynamic_configuration(&mut spice_instance)?;

    println!("\n✅ CLI application configuration example completed!");
    Ok(())
}

fn build_cli() -> Command {
    Command::new("cli-processor")
        .version("1.0.0")
        .about("A CLI application demonstrating comprehensive configuration management")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
                .action(clap::ArgAction::Set),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Enable verbose output")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .help("Suppress output")
                .action(clap::ArgAction::SetTrue)
                .conflicts_with("verbose"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .help("Show what would be done without executing")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("input")
                .short('i')
                .long("input")
                .value_name("FILES")
                .help("Input files or directories")
                .action(clap::ArgAction::Append),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("FILE")
                .help("Output file")
                .action(clap::ArgAction::Set),
        )
        .arg(
            Arg::new("format")
                .short('f')
                .long("format")
                .value_name("FORMAT")
                .help("Output format (json, yaml, xml)")
                .action(clap::ArgAction::Set),
        )
        .arg(
            Arg::new("workers")
                .short('w')
                .long("workers")
                .value_name("NUM")
                .help("Number of worker threads")
                .action(clap::ArgAction::Set),
        )
        .arg(
            Arg::new("parallel")
                .short('p')
                .long("parallel")
                .help("Enable parallel processing")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("recursive")
                .short('r')
                .long("recursive")
                .help("Process directories recursively")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("overwrite")
                .long("overwrite")
                .help("Overwrite existing output files")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("log-level")
                .long("log-level")
                .value_name("LEVEL")
                .help("Logging level (debug, info, warn, error)")
                .action(clap::ArgAction::Set),
        )
        .arg(
            Arg::new("log-file")
                .long("log-file")
                .value_name("FILE")
                .help("Log to file instead of stdout")
                .action(clap::ArgAction::Set),
        )
}

fn setup_configuration(matches: &ArgMatches) -> Result<Spice, Box<dyn std::error::Error>> {
    let mut spice_instance = Spice::new();

    println!("\n📋 Setting up configuration layers...");

    // 1. Set up defaults (lowest precedence)
    setup_defaults(&mut spice_instance)?;
    println!("   ✓ Default values configured");

    // 2. Load configuration file
    setup_config_file(&mut spice_instance, matches)?;

    // 3. Set up environment variables
    setup_environment_variables(&mut spice_instance)?;
    println!("   ✓ Environment variables configured");

    // 4. Apply command line arguments (highest precedence)
    apply_cli_arguments(&mut spice_instance, matches)?;
    println!("   ✓ Command line arguments applied");

    Ok(spice_instance)
}

fn setup_defaults(spice_instance: &mut Spice) -> Result<(), Box<dyn std::error::Error>> {
    // Application defaults
    spice_instance.set_default("name", ConfigValue::from(default_app_name()))?;
    spice_instance.set_default("version", ConfigValue::from(default_version()))?;
    spice_instance.set_default("verbose", ConfigValue::from(false))?;
    spice_instance.set_default("quiet", ConfigValue::from(false))?;
    spice_instance.set_default("dry_run", ConfigValue::from(false))?;

    // Input defaults
    spice_instance.set_default("input.format", ConfigValue::from(default_input_format()))?;
    spice_instance.set_default("input.recursive", ConfigValue::from(false))?;
    spice_instance.set_default("input.follow_symlinks", ConfigValue::from(false))?;

    // Output defaults
    spice_instance.set_default("output.format", ConfigValue::from(default_output_format()))?;
    spice_instance.set_default("output.overwrite", ConfigValue::from(false))?;
    spice_instance.set_default("output.create_dirs", ConfigValue::from(true))?;
    spice_instance.set_default(
        "output.compression",
        ConfigValue::from(default_compression()),
    )?;

    // Processing defaults
    spice_instance.set_default(
        "processing.workers",
        ConfigValue::from(default_workers() as i64),
    )?;
    spice_instance.set_default(
        "processing.batch_size",
        ConfigValue::from(default_batch_size() as i64),
    )?;
    spice_instance.set_default(
        "processing.timeout",
        ConfigValue::from(default_timeout() as i64),
    )?;
    spice_instance.set_default("processing.parallel", ConfigValue::from(false))?;

    // Logging defaults
    spice_instance.set_default("logging.level", ConfigValue::from(default_log_level()))?;
    spice_instance.set_default("logging.timestamp", ConfigValue::from(true))?;
    spice_instance.set_default("logging.colors", ConfigValue::from(true))?;

    Ok(())
}

fn setup_config_file(
    spice_instance: &mut Spice,
    matches: &ArgMatches,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check for explicit config file from command line
    if let Some(config_file) = matches.get_one::<String>("config") {
        match spice_instance.set_config_file(config_file) {
            Ok(()) => {
                println!("   ✓ Loaded configuration from: {}", config_file);
                return Ok(());
            }
            Err(e) => {
                eprintln!("   ⚠ Failed to load config file '{}': {}", config_file, e);
                return Err(e.into());
            }
        }
    }

    // Try to find configuration file in standard locations
    spice_instance.add_config_path(".");
    spice_instance.add_config_path("./config");
    spice_instance.add_config_path("~/.config/cli-processor");
    spice_instance.add_config_path("/etc/cli-processor");

    spice_instance.set_config_name("config");
    match spice_instance.read_in_config() {
        Ok(()) => {
            println!("   ✓ Found and loaded configuration file");
        }
        Err(_) => {
            println!("   ⚠ No configuration file found, creating sample...");
            create_sample_config()?;
        }
    }

    Ok(())
}

fn setup_environment_variables(
    spice_instance: &mut Spice,
) -> Result<(), Box<dyn std::error::Error>> {
    // Add environment layer with CLI_PROCESSOR prefix
    let env_layer = EnvConfigLayer::new(Some("CLI_PROCESSOR".to_string()), true);
    spice_instance.add_layer(Box::new(env_layer));

    // Show recognized environment variables
    let env_vars = [
        "CLI_PROCESSOR_VERBOSE",
        "CLI_PROCESSOR_OUTPUT_FORMAT",
        "CLI_PROCESSOR_PROCESSING_WORKERS",
        "CLI_PROCESSOR_LOGGING_LEVEL",
    ];

    let mut found_vars = Vec::new();
    for var in &env_vars {
        if env::var(var).is_ok() {
            found_vars.push(*var);
        }
    }

    if !found_vars.is_empty() {
        println!("     Found environment variables: {:?}", found_vars);
    }

    Ok(())
}

fn apply_cli_arguments(
    spice_instance: &mut Spice,
    matches: &ArgMatches,
) -> Result<(), Box<dyn std::error::Error>> {
    // Apply command line arguments with highest precedence

    if matches.get_flag("verbose") {
        spice_instance.set("verbose", ConfigValue::from(true))?;
    }

    if matches.get_flag("quiet") {
        spice_instance.set("quiet", ConfigValue::from(true))?;
    }

    if matches.get_flag("dry-run") {
        spice_instance.set("dry_run", ConfigValue::from(true))?;
    }

    if let Some(inputs) = matches.get_many::<String>("input") {
        let input_files: Vec<ConfigValue> = inputs.map(|s| ConfigValue::from(s.clone())).collect();
        spice_instance.set("input.files", ConfigValue::Array(input_files))?;
    }

    if let Some(output) = matches.get_one::<String>("output") {
        spice_instance.set("output.file", ConfigValue::from(output.clone()))?;
    }

    if let Some(format) = matches.get_one::<String>("format") {
        spice_instance.set("output.format", ConfigValue::from(format.clone()))?;
    }

    if let Some(workers) = matches.get_one::<String>("workers") {
        if let Ok(num) = workers.parse::<i64>() {
            spice_instance.set("processing.workers", ConfigValue::from(num))?;
        }
    }

    if matches.get_flag("parallel") {
        spice_instance.set("processing.parallel", ConfigValue::from(true))?;
    }

    if matches.get_flag("recursive") {
        spice_instance.set("input.recursive", ConfigValue::from(true))?;
    }

    if matches.get_flag("overwrite") {
        spice_instance.set("output.overwrite", ConfigValue::from(true))?;
    }

    if let Some(log_level) = matches.get_one::<String>("log-level") {
        spice_instance.set("logging.level", ConfigValue::from(log_level.clone()))?;
    }

    if let Some(log_file) = matches.get_one::<String>("log-file") {
        spice_instance.set("logging.file", ConfigValue::from(log_file.clone()))?;
    }

    Ok(())
}

fn display_configuration(config: &CliAppConfig, matches: &ArgMatches) {
    println!("\n🔧 Final Configuration:");
    println!("======================");

    println!("Application: {} v{}", config.name, config.version);
    println!(
        "Mode: {}",
        if config.dry_run { "DRY RUN" } else { "EXECUTE" }
    );
    println!(
        "Verbosity: {}",
        match (config.verbose, config.quiet) {
            (true, _) => "VERBOSE",
            (_, true) => "QUIET",
            _ => "NORMAL",
        }
    );

    println!("\nInput Configuration:");
    println!("  Files: {:?}", config.input.files);
    println!("  Format: {}", config.input.format);
    println!("  Recursive: {}", config.input.recursive);
    println!("  Follow Symlinks: {}", config.input.follow_symlinks);
    if !config.input.include_patterns.is_empty() {
        println!("  Include Patterns: {:?}", config.input.include_patterns);
    }
    if !config.input.exclude_patterns.is_empty() {
        println!("  Exclude Patterns: {:?}", config.input.exclude_patterns);
    }

    println!("\nOutput Configuration:");
    if let Some(file) = &config.output.file {
        println!("  File: {}", file);
    } else {
        println!("  File: <stdout>");
    }
    println!("  Format: {}", config.output.format);
    println!("  Overwrite: {}", config.output.overwrite);
    println!("  Create Directories: {}", config.output.create_dirs);
    println!("  Compression: {}", config.output.compression);

    println!("\nProcessing Configuration:");
    println!("  Workers: {}", config.processing.workers);
    println!("  Batch Size: {}", config.processing.batch_size);
    println!("  Timeout: {}s", config.processing.timeout);
    println!("  Parallel: {}", config.processing.parallel);
    if let Some(limit) = config.processing.memory_limit {
        println!("  Memory Limit: {} bytes", limit);
    }
    if !config.processing.options.is_empty() {
        println!("  Options: {:?}", config.processing.options);
    }

    println!("\nLogging Configuration:");
    println!("  Level: {}", config.logging.level);
    if let Some(file) = &config.logging.file {
        println!("  File: {}", file);
    } else {
        println!("  File: <stdout>");
    }
    println!("  Timestamp: {}", config.logging.timestamp);
    println!("  Colors: {}", config.logging.colors);

    // Show configuration sources
    println!("\nConfiguration Sources:");
    if matches.contains_id("config") {
        println!("  ✓ Explicit config file");
    }
    if env::var("CLI_PROCESSOR_VERBOSE").is_ok() {
        println!("  ✓ Environment variables");
    }
    if matches.args_present() {
        println!("  ✓ Command line arguments");
    }
}

fn demonstrate_application_logic(config: &CliAppConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🚀 Demonstrating Application Logic:");
    println!("===================================");

    // Initialize logging based on configuration
    setup_logging(config);

    // Validate configuration
    validate_configuration(config)?;

    // Process input files
    process_files(config)?;

    // Generate output
    generate_output(config)?;

    Ok(())
}

fn setup_logging(config: &CliAppConfig) {
    println!("📝 Setting up logging:");
    println!("   Level: {}", config.logging.level);

    if let Some(file) = &config.logging.file {
        println!("   Output: {} (file)", file);
    } else {
        println!("   Output: stdout");
    }

    println!(
        "   Timestamp: {}",
        if config.logging.timestamp {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!(
        "   Colors: {}",
        if config.logging.colors {
            "enabled"
        } else {
            "disabled"
        }
    );
}

fn validate_configuration(config: &CliAppConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n✅ Validating configuration:");

    // Check for conflicting options
    if config.verbose && config.quiet {
        return Err("Cannot be both verbose and quiet".into());
    }

    // Validate worker count
    if config.processing.workers == 0 {
        return Err("Worker count must be greater than 0".into());
    }

    if config.processing.workers > 64 {
        println!(
            "   ⚠ Warning: High worker count ({}), consider reducing",
            config.processing.workers
        );
    }

    // Validate output format
    let valid_formats = ["json", "yaml", "xml", "csv"];
    if !valid_formats.contains(&config.output.format.as_str()) {
        return Err(format!("Invalid output format: {}", config.output.format).into());
    }

    // Validate log level
    let valid_levels = ["debug", "info", "warn", "error"];
    if !valid_levels.contains(&config.logging.level.as_str()) {
        return Err(format!("Invalid log level: {}", config.logging.level).into());
    }

    // Check input files exist (in real app)
    if config.input.files.is_empty() {
        println!("   ⚠ Warning: No input files specified");
    }

    println!("   ✓ Configuration is valid");
    Ok(())
}

fn process_files(config: &CliAppConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📁 Processing files:");

    if config.input.files.is_empty() {
        println!("   ⚠ No files to process");
        return Ok(());
    }

    for file in &config.input.files {
        if config.dry_run {
            println!("   [DRY RUN] Would process: {}", file);
        } else {
            println!("   Processing: {}", file);
            // In a real application, you would process the file here
        }
    }

    if config.processing.parallel {
        println!(
            "   Using {} workers for parallel processing",
            config.processing.workers
        );
    }

    println!("   Batch size: {}", config.processing.batch_size);
    println!("   Timeout: {}s", config.processing.timeout);

    Ok(())
}

fn generate_output(config: &CliAppConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📤 Generating output:");

    if let Some(output_file) = &config.output.file {
        if config.dry_run {
            println!("   [DRY RUN] Would write to: {}", output_file);
        } else {
            println!("   Writing to: {}", output_file);

            if PathBuf::from(output_file).exists() && !config.output.overwrite {
                return Err(format!(
                    "Output file '{}' exists and overwrite is disabled",
                    output_file
                )
                .into());
            }
        }
    } else {
        println!("   Writing to: stdout");
    }

    println!("   Format: {}", config.output.format);
    println!("   Compression: {}", config.output.compression);

    Ok(())
}

fn demonstrate_dynamic_configuration(
    spice_instance: &mut Spice,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔄 Dynamic Configuration:");
    println!("=========================");

    // Show how to update configuration at runtime
    println!("Updating configuration at runtime...");

    spice_instance.set("processing.workers", ConfigValue::from(8i64))?;
    spice_instance.set("logging.level", ConfigValue::from("debug"))?;

    let updated_workers = spice_instance.get_int("processing.workers")?.unwrap_or(0);
    let updated_log_level = spice_instance
        .get_string("logging.level")?
        .unwrap_or_default();

    println!("   Updated workers: {}", updated_workers);
    println!("   Updated log level: {}", updated_log_level);

    // Show how to watch for configuration file changes
    match spice_instance.watch_config() {
        Ok(()) => {
            println!("   ✓ Configuration file watching enabled");

            spice_instance.on_config_change(Box::new(|| {
                println!("   🔄 Configuration file changed - reloading...");
            }))?;
        }
        Err(e) => {
            println!("   ⚠ Could not enable file watching: {}", e);
        }
    }

    Ok(())
}

fn create_sample_config() -> Result<(), Box<dyn std::error::Error>> {
    let sample_config = r#"{
  "name": "cli-processor",
  "version": "1.0.0",
  "verbose": false,
  "quiet": false,
  "dry_run": false,
  "input": {
    "format": "auto",
    "recursive": false,
    "follow_symlinks": false,
    "include_patterns": ["*.txt", "*.json"],
    "exclude_patterns": ["*.tmp", "*.bak"]
  },
  "output": {
    "format": "json",
    "overwrite": false,
    "create_dirs": true,
    "compression": "gzip"
  },
  "processing": {
    "workers": 4,
    "batch_size": 100,
    "timeout": 30,
    "parallel": true,
    "memory_limit": 1073741824,
    "options": {
      "strict_mode": "true",
      "validate_input": "true"
    }
  },
  "logging": {
    "level": "info",
    "file": "/var/log/cli-processor.log",
    "timestamp": true,
    "colors": false
  }
}"#;

    fs::write("config.json", sample_config)?;
    println!("   ✓ Created sample config.json");
    Ok(())
}
