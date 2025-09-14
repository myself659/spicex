//! Core Spice configuration management struct and implementation.

use crate::default_layer::DefaultConfigLayer;
use crate::error::{ConfigError, ConfigResult};
use crate::file_layer::FileConfigLayer;
use crate::layer::{utils, ConfigLayer, LayerPriority};
use crate::value::ConfigValue;
use crate::watcher::FileWatcher;
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};

/// Represents a component of a configuration key path.
#[derive(Debug, Clone, PartialEq)]
enum KeyPart {
    /// A string key for object access
    Key(String),
    /// A numeric index for array access
    Index(usize),
}

/// The main Spice configuration manager.
///
/// This struct manages configuration from multiple sources with a clear precedence hierarchy.
pub struct Spice {
    /// Configuration layers ordered by precedence (highest first)
    layers: Vec<Box<dyn ConfigLayer>>,

    /// Configuration file search paths
    config_paths: Vec<PathBuf>,

    /// Configuration file name (without extension)
    config_name: String,

    /// Environment variable prefix
    env_prefix: Option<String>,

    /// Key delimiter for nested access
    key_delimiter: String,

    /// Whether to automatically bind environment variables
    automatic_env: bool,

    /// File watcher for configuration file changes
    watcher: Option<FileWatcher>,

    /// List of configuration files being watched
    watched_config_files: Vec<PathBuf>,

    /// Channel receiver for reload signals from file watcher
    reload_receiver: Option<mpsc::Receiver<()>>,

    /// Flag to track if auto-reload callback is registered
    auto_reload_registered: bool,

    /// Flag to indicate if configuration needs to be reloaded
    needs_reload: Arc<std::sync::atomic::AtomicBool>,

    /// User callbacks to trigger after successful configuration reload
    user_callbacks: Vec<Box<dyn Fn() + Send + Sync>>,
}

impl Spice {
    /// Creates a new Spice instance with default settings.
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
            config_paths: Vec::new(),
            config_name: String::new(),
            env_prefix: None,
            key_delimiter: ".".to_string(),
            automatic_env: false,
            watcher: None,
            watched_config_files: Vec::new(),
            reload_receiver: None,
            auto_reload_registered: false,
            needs_reload: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            user_callbacks: Vec::new(),
        }
    }

    /// Adds a configuration layer to the Spice instance.
    /// Layers are automatically sorted by priority after addition.
    ///
    /// # Arguments
    /// * `layer` - The configuration layer to add
    ///
    /// # Example
    /// ```
    /// use spicex::{Spice, FileConfigLayer};
    /// use std::path::PathBuf;
    ///
    /// let mut spice = Spice::new();
    /// // Note: FileConfigLayer creation will be available after file layer implementation
    /// ```
    pub fn add_layer(&mut self, layer: Box<dyn ConfigLayer>) {
        self.layers.push(layer);
        utils::sort_layers_by_priority(&mut self.layers);
    }

    /// Removes all layers with the specified priority.
    ///
    /// # Arguments
    /// * `priority` - The priority level of layers to remove
    ///
    /// # Returns
    /// The number of layers removed
    pub fn remove_layers_by_priority(&mut self, priority: LayerPriority) -> usize {
        let initial_len = self.layers.len();
        self.layers.retain(|layer| layer.priority() != priority);
        initial_len - self.layers.len()
    }

    /// Returns the number of configuration layers currently registered.
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// Returns a list of all layer source names and their priorities.
    pub fn layer_info(&self) -> Vec<(String, LayerPriority)> {
        self.layers
            .iter()
            .map(|layer| (layer.source_name().to_string(), layer.priority()))
            .collect()
    }

    /// Clears all configuration layers.
    pub fn clear_layers(&mut self) {
        self.layers.clear();
    }

    /// Sets the configuration file name (without extension).
    ///
    /// # Arguments
    /// * `name` - The configuration file name
    pub fn set_config_name(&mut self, name: impl Into<String>) {
        self.config_name = name.into();
    }

    /// Gets the current configuration file name.
    pub fn config_name(&self) -> &str {
        &self.config_name
    }

    /// Adds a path to search for configuration files.
    ///
    /// # Arguments
    /// * `path` - The path to add to the search list
    pub fn add_config_path(&mut self, path: impl Into<PathBuf>) {
        self.config_paths.push(path.into());
    }

    /// Gets all configuration search paths.
    pub fn config_paths(&self) -> &[PathBuf] {
        &self.config_paths
    }

    /// Searches for configuration files in the configured search paths.
    /// Returns the first configuration file found that matches the configured name.
    ///
    /// # Returns
    /// * `ConfigResult<Option<PathBuf>>` - The path to the found configuration file, or None if not found
    ///
    /// # Example
    /// ```
    /// use spicex::Spice;
    /// use std::path::PathBuf;
    ///
    /// let mut spice = Spice::new();
    /// spice.set_config_name("config");
    /// spice.add_config_path("./configs");
    /// spice.add_config_path("/etc/myapp");
    ///
    /// // This will search for config.json, config.yaml, config.toml, config.ini
    /// // in ./configs and /etc/myapp directories
    /// if let Some(config_file) = spice.find_config_file().unwrap() {
    ///     println!("Found config file: {}", config_file.display());
    /// }
    /// ```
    pub fn find_config_file(&self) -> ConfigResult<Option<PathBuf>> {
        if self.config_name.is_empty() {
            return Ok(None);
        }

        let supported_extensions = ["json", "yaml", "yml", "toml", "ini"];

        // Search in configured paths first
        for search_path in &self.config_paths {
            for extension in &supported_extensions {
                let config_file = search_path.join(format!("{}.{}", self.config_name, extension));
                if config_file.exists() && config_file.is_file() {
                    return Ok(Some(config_file));
                }
            }
        }

        // If no paths configured or file not found, search in standard locations
        if self.config_paths.is_empty() {
            let standard_paths = self.get_standard_config_paths()?;
            for search_path in standard_paths {
                for extension in &supported_extensions {
                    let config_file =
                        search_path.join(format!("{}.{}", self.config_name, extension));
                    if config_file.exists() && config_file.is_file() {
                        return Ok(Some(config_file));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Gets standard configuration directory paths based on the operating system.
    ///
    /// # Returns
    /// * `ConfigResult<Vec<PathBuf>>` - List of standard configuration directories
    fn get_standard_config_paths(&self) -> ConfigResult<Vec<PathBuf>> {
        let mut paths = Vec::new();

        // Current directory (highest priority)
        paths.push(PathBuf::from("."));

        // User's home directory
        if let Some(home_dir) = dirs::home_dir() {
            paths.push(home_dir.join(".config"));
            paths.push(home_dir);
        }

        // System-wide configuration directories
        #[cfg(unix)]
        {
            paths.push(PathBuf::from("/etc"));
            paths.push(PathBuf::from("/usr/local/etc"));
        }

        #[cfg(windows)]
        {
            if let Ok(program_data) = env::var("PROGRAMDATA") {
                paths.push(PathBuf::from(program_data));
            }
            if let Ok(app_data) = env::var("APPDATA") {
                paths.push(PathBuf::from(app_data));
            }
        }

        Ok(paths)
    }

    /// Searches for all configuration files with the given name in search paths.
    /// Returns all matching files found, ordered by search path priority.
    ///
    /// # Returns
    /// * `ConfigResult<Vec<PathBuf>>` - List of all found configuration files
    ///
    /// # Example
    /// ```
    /// use spicex::Spice;
    ///
    /// let mut spice = Spice::new();
    /// spice.set_config_name("config");
    /// spice.add_config_path("./configs");
    /// spice.add_config_path("/etc/myapp");
    ///
    /// let all_configs = spice.find_all_config_files().unwrap();
    /// for config_file in all_configs {
    ///     println!("Found config: {}", config_file.display());
    /// }
    /// ```
    pub fn find_all_config_files(&self) -> ConfigResult<Vec<PathBuf>> {
        if self.config_name.is_empty() {
            return Ok(Vec::new());
        }

        let mut found_files = Vec::new();
        let supported_extensions = ["json", "yaml", "yml", "toml", "ini"];

        // Search in configured paths first
        let search_paths = if self.config_paths.is_empty() {
            self.get_standard_config_paths()?
        } else {
            self.config_paths.clone()
        };

        for search_path in search_paths {
            for extension in &supported_extensions {
                let config_file = search_path.join(format!("{}.{}", self.config_name, extension));
                if config_file.exists() && config_file.is_file() {
                    found_files.push(config_file);
                }
            }
        }

        Ok(found_files)
    }

    /// Automatically discovers and loads a configuration file.
    /// This method searches for configuration files using the configured name and paths,
    /// then loads the first file found.
    ///
    /// # Returns
    /// * `ConfigResult<()>` - Success if a file was found and loaded, or an error
    ///
    /// # Errors
    /// * `ConfigError::KeyNotFound` - If no configuration file is found
    /// * `ConfigError::Io` - If the file cannot be read
    /// * `ConfigError::Parse` - If the file content cannot be parsed
    ///
    /// # Example
    /// ```
    /// use spicex::Spice;
    ///
    /// let mut spice = Spice::new();
    /// spice.set_config_name("config");
    /// spice.add_config_path("./configs");
    ///
    /// // This will automatically find and load the first config file found
    /// match spice.read_in_config() {
    ///     Ok(()) => println!("Configuration loaded successfully"),
    ///     Err(e) => println!("Failed to load configuration: {}", e),
    /// }
    /// ```
    pub fn read_in_config(&mut self) -> ConfigResult<()> {
        let config_file = self.find_config_file()?.ok_or_else(|| {
            ConfigError::key_not_found(format!("configuration file '{}'", self.config_name))
        })?;

        self.load_config_file(config_file)
    }

    /// Loads a specific configuration file and adds it as a configuration layer.
    ///
    /// # Arguments
    /// * `config_file` - Path to the configuration file to load
    ///
    /// # Returns
    /// * `ConfigResult<()>` - Success if the file was loaded, or an error
    pub fn load_config_file<P: AsRef<Path>>(&mut self, config_file: P) -> ConfigResult<()> {
        let file_layer = FileConfigLayer::new(config_file)?;
        self.add_layer(Box::new(file_layer));
        Ok(())
    }

    /// Merges multiple configuration files into the current configuration.
    /// This method finds all configuration files with the configured name and merges them
    /// in order of discovery (first found has highest precedence).
    ///
    /// # Returns
    /// * `ConfigResult<usize>` - The number of configuration files merged
    ///
    /// # Example
    /// ```
    /// use spicex::Spice;
    ///
    /// let mut spice = Spice::new();
    /// spice.set_config_name("config");
    /// spice.add_config_path("./configs");
    /// spice.add_config_path("/etc/myapp");
    ///
    /// // This will find and merge all config files found in search paths
    /// let merged_count = spice.merge_in_config().unwrap();
    /// println!("Merged {} configuration files", merged_count);
    /// ```
    pub fn merge_in_config(&mut self) -> ConfigResult<usize> {
        let config_files = self.find_all_config_files()?;
        let count = config_files.len();

        for config_file in config_files {
            self.load_config_file(config_file)?;
        }

        Ok(count)
    }

    /// Sets the configuration file path explicitly and loads it.
    /// This method bypasses the search mechanism and loads a specific file.
    ///
    /// # Arguments
    /// * `config_file` - Path to the configuration file
    ///
    /// # Returns
    /// * `ConfigResult<()>` - Success if the file was loaded, or an error
    ///
    /// # Example
    /// ```no_run
    /// use spicex::Spice;
    ///
    /// let mut spice = Spice::new();
    /// spice.set_config_file("./my-config.json").unwrap();
    /// ```
    pub fn set_config_file<P: AsRef<Path>>(&mut self, config_file: P) -> ConfigResult<()> {
        self.load_config_file(config_file)
    }

    /// Sets the environment variable prefix.
    ///
    /// # Arguments
    /// * `prefix` - The prefix to use for environment variables
    pub fn set_env_prefix(&mut self, prefix: impl Into<String>) {
        self.env_prefix = Some(prefix.into());
    }

    /// Gets the current environment variable prefix.
    pub fn env_prefix(&self) -> Option<&str> {
        self.env_prefix.as_deref()
    }

    /// Sets whether to automatically bind environment variables.
    ///
    /// # Arguments
    /// * `automatic` - Whether to enable automatic environment variable binding
    pub fn set_automatic_env(&mut self, automatic: bool) {
        self.automatic_env = automatic;
    }

    /// Gets whether automatic environment variable binding is enabled.
    pub fn is_automatic_env(&self) -> bool {
        self.automatic_env
    }

    /// Binds command line flags to the configuration.
    /// This method adds a FlagConfigLayer with the provided clap ArgMatches.
    ///
    /// # Arguments
    /// * `matches` - The parsed command line arguments from clap
    ///
    /// # Example
    /// ```
    /// use spicex::Spice;
    /// use clap::{Arg, Command};
    ///
    /// let app = Command::new("myapp")
    ///     .arg(Arg::new("host")
    ///         .long("host")
    ///         .value_name("HOST"));
    ///
    /// let args = vec!["myapp", "--host", "localhost"];
    /// let matches = app.try_get_matches_from(args).unwrap();
    ///
    /// let mut spice = Spice::new();
    /// spice.bind_flags(matches);
    /// ```
    #[cfg(feature = "cli")]
    pub fn bind_flags(&mut self, matches: clap::ArgMatches) {
        use crate::cli::FlagConfigLayer;
        let flag_layer = FlagConfigLayer::new(matches);
        self.add_layer(Box::new(flag_layer));
    }

    /// Binds command line flags with custom flag-to-key mappings.
    ///
    /// # Arguments
    /// * `matches` - The parsed command line arguments from clap
    /// * `mappings` - HashMap mapping flag names to configuration keys
    ///
    /// # Example
    /// ```
    /// use spicex::Spice;
    /// use clap::{Arg, Command};
    /// use std::collections::HashMap;
    ///
    /// let app = Command::new("myapp")
    ///     .arg(Arg::new("db_host")
    ///         .long("db-host")
    ///         .value_name("HOST"));
    ///
    /// let args = vec!["myapp", "--db-host", "localhost"];
    /// let matches = app.try_get_matches_from(args).unwrap();
    ///
    /// let mut mappings = HashMap::new();
    /// mappings.insert("db_host".to_string(), "database.host".to_string());
    ///
    /// let mut spice = Spice::new();
    /// spice.bind_flags_with_mappings(matches, mappings);
    /// ```
    #[cfg(feature = "cli")]
    pub fn bind_flags_with_mappings(
        &mut self,
        matches: clap::ArgMatches,
        mappings: std::collections::HashMap<String, String>,
    ) {
        use crate::cli::FlagConfigLayer;
        let flag_layer = FlagConfigLayer::with_mappings(matches, mappings);
        self.add_layer(Box::new(flag_layer));
    }

    /// Binds a specific flag to a configuration key.
    /// This is useful when you want to bind individual flags after the initial setup.
    ///
    /// # Arguments
    /// * `flag_name` - The name of the command line flag
    /// * `config_key` - The configuration key to bind to
    ///
    /// # Returns
    /// * `ConfigResult<()>` - Ok if successful, error if no flag layer exists
    ///
    /// # Example
    /// ```
    /// use spicex::Spice;
    /// use clap::{Arg, Command};
    ///
    /// let app = Command::new("myapp")
    ///     .arg(Arg::new("verbose")
    ///         .long("verbose")
    ///         .action(clap::ArgAction::SetTrue));
    ///
    /// let args = vec!["myapp", "--verbose"];
    /// let matches = app.try_get_matches_from(args).unwrap();
    ///
    /// let mut spice = Spice::new();
    /// spice.bind_flags(matches);
    /// spice.bind_flag("verbose", "logging.verbose").unwrap();
    /// ```
    #[cfg(feature = "cli")]
    pub fn bind_flag(
        &mut self,
        flag_name: impl Into<String>,
        config_key: impl Into<String>,
    ) -> ConfigResult<()> {
        use crate::cli::FlagConfigLayer;

        // Find the flag layer and add the mapping
        for layer in &mut self.layers {
            if layer.priority() == LayerPriority::Flags {
                if let Some(flag_layer) = layer.as_any_mut().downcast_mut::<FlagConfigLayer>() {
                    flag_layer.add_flag_mapping(flag_name, config_key);
                    return Ok(());
                }
            }
        }

        Err(ConfigError::unsupported_operation(
            "No flag configuration layer found. Call bind_flags() first.",
        ))
    }

    /// Sets the key delimiter for nested access.
    ///
    /// # Arguments
    /// * `delimiter` - The delimiter to use (default is ".")
    pub fn set_key_delimiter(&mut self, delimiter: impl Into<String>) {
        self.key_delimiter = delimiter.into();
    }

    /// Gets the current key delimiter.
    pub fn key_delimiter(&self) -> &str {
        &self.key_delimiter
    }

    /// Gets a configuration value by key, searching through all layers by precedence.
    /// Supports dot notation for nested access (e.g., "database.host") and array indexing (e.g., "servers.0.host").
    ///
    /// # Arguments
    /// * `key` - The configuration key to retrieve, supporting dot notation for nested access
    ///
    /// # Returns
    /// * `ConfigResult<Option<ConfigValue>>` - The configuration value if found, None if not found
    ///
    /// # Example
    /// ```
    /// use spicex::{Spice, ConfigValue};
    ///
    /// let spice = Spice::new();
    /// // After adding layers with configuration data
    /// // let value = spice.get("database.host").unwrap();
    /// // let array_value = spice.get("servers.0.host").unwrap();
    /// ```
    pub fn get(&self, key: &str) -> ConfigResult<Option<ConfigValue>> {
        // First try to get the exact key from layers
        if let Some(value) = utils::merge_value_from_layers(&self.layers, key)? {
            return Ok(Some(value));
        }

        // If not found and key contains delimiter, try nested access
        if key.contains(&self.key_delimiter) {
            self.get_nested(key)
        } else {
            Ok(None)
        }
    }

    /// Gets a nested configuration value using dot notation.
    /// This method handles nested object access and array indexing.
    ///
    /// # Arguments
    /// * `key` - The nested key path (e.g., "database.host", "servers.0.port")
    ///
    /// # Returns
    /// * `ConfigResult<Option<ConfigValue>>` - The nested value if found
    fn get_nested(&self, key: &str) -> ConfigResult<Option<ConfigValue>> {
        let key_parts = self.parse_key(key);

        // Try to find a root key that matches the beginning of our path
        for i in (1..=key_parts.len()).rev() {
            let root_key = self.key_parts_to_string(&key_parts[..i]);

            if let Some(root_value) = utils::merge_value_from_layers(&self.layers, &root_key)? {
                if i == key_parts.len() {
                    // Exact match
                    return Ok(Some(root_value));
                } else {
                    // Need to traverse deeper
                    let remaining_path = &key_parts[i..];
                    return Ok(self.traverse_nested_value(&root_value, remaining_path));
                }
            }
        }

        Ok(None)
    }

    /// Parses a key into its component parts, handling array indices.
    ///
    /// # Arguments
    /// * `key` - The key to parse
    ///
    /// # Returns
    /// * `Vec<KeyPart>` - The parsed key components
    fn parse_key(&self, key: &str) -> Vec<KeyPart> {
        key.split(&self.key_delimiter)
            .map(|part| {
                // Check if this part is an array index
                if let Ok(index) = part.parse::<usize>() {
                    KeyPart::Index(index)
                } else {
                    KeyPart::Key(part.to_string())
                }
            })
            .collect()
    }

    /// Traverses a nested ConfigValue using the provided path.
    ///
    /// # Arguments
    /// * `value` - The root value to traverse
    /// * `path` - The remaining path components
    ///
    /// # Returns
    /// * `Option<ConfigValue>` - The value at the end of the path, if found
    fn traverse_nested_value(&self, value: &ConfigValue, path: &[KeyPart]) -> Option<ConfigValue> {
        if path.is_empty() {
            return Some(value.clone());
        }

        match (&path[0], value) {
            (KeyPart::Key(key), ConfigValue::Object(obj)) => {
                if let Some(nested_value) = obj.get(key) {
                    self.traverse_nested_value(nested_value, &path[1..])
                } else {
                    None
                }
            }
            (KeyPart::Index(index), ConfigValue::Array(arr)) => {
                if *index < arr.len() {
                    self.traverse_nested_value(&arr[*index], &path[1..])
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Converts a slice of KeyPart back to a string key.
    ///
    /// # Arguments
    /// * `parts` - The key parts to convert
    ///
    /// # Returns
    /// * `String` - The reconstructed key string
    fn key_parts_to_string(&self, parts: &[KeyPart]) -> String {
        parts
            .iter()
            .map(|part| match part {
                KeyPart::Key(key) => key.clone(),
                KeyPart::Index(index) => index.to_string(),
            })
            .collect::<Vec<String>>()
            .join(&self.key_delimiter)
    }

    /// Sets a configuration value explicitly (highest precedence).
    /// This creates or updates an explicit layer with the highest precedence.
    ///
    /// # Arguments
    /// * `key` - The configuration key to set
    /// * `value` - The configuration value to set
    ///
    /// # Example
    /// ```
    /// use spicex::{Spice, ConfigValue};
    ///
    /// let mut spice = Spice::new();
    /// spice.set("database.host", ConfigValue::from("localhost")).unwrap();
    /// ```
    pub fn set(&mut self, key: &str, value: ConfigValue) -> ConfigResult<()> {
        // Find or create an explicit layer
        let explicit_layer_index = self
            .layers
            .iter()
            .position(|layer| layer.priority() == LayerPriority::Explicit);

        match explicit_layer_index {
            Some(index) => {
                // Update existing explicit layer
                let layer = &mut self.layers[index];
                layer.set(key, value)?;
            }
            None => {
                // Create new explicit layer
                let mut explicit_layer = ExplicitConfigLayer::new();
                explicit_layer.set(key, value)?;
                self.add_layer(Box::new(explicit_layer));
            }
        }

        Ok(())
    }

    /// Sets a default configuration value.
    /// Default values have the lowest precedence and will only be used if no other
    /// configuration source provides a value for the same key.
    ///
    /// # Arguments
    /// * `key` - The configuration key to set a default for
    /// * `value` - The default configuration value
    ///
    /// # Example
    /// ```
    /// use spicex::{Spice, ConfigValue};
    ///
    /// let mut spice = Spice::new();
    /// spice.set_default("database.host", ConfigValue::from("localhost")).unwrap();
    /// spice.set_default("database.port", ConfigValue::from(5432i64)).unwrap();
    ///
    /// // These defaults will be used unless overridden by other configuration sources
    /// assert_eq!(spice.get_string("database.host").unwrap(), Some("localhost".to_string()));
    /// ```
    pub fn set_default(&mut self, key: &str, value: ConfigValue) -> ConfigResult<()> {
        // Find or create a default layer
        let default_layer_index = self
            .layers
            .iter()
            .position(|layer| layer.priority() == LayerPriority::Defaults);

        match default_layer_index {
            Some(index) => {
                // Update existing default layer
                let layer = &mut self.layers[index];
                layer.set(key, value)?;
            }
            None => {
                // Create new default layer
                let mut default_layer = DefaultConfigLayer::new();
                default_layer.set(key, value)?;
                self.add_layer(Box::new(default_layer));
            }
        }

        Ok(())
    }

    /// Sets multiple default configuration values at once.
    /// This is more efficient than calling set_default multiple times.
    ///
    /// # Arguments
    /// * `defaults` - A HashMap containing the default key-value pairs
    ///
    /// # Example
    /// ```
    /// use spicex::{Spice, ConfigValue};
    /// use std::collections::HashMap;
    ///
    /// let mut spice = Spice::new();
    /// let mut defaults = HashMap::new();
    /// defaults.insert("database.host".to_string(), ConfigValue::from("localhost"));
    /// defaults.insert("database.port".to_string(), ConfigValue::from(5432i64));
    /// defaults.insert("database.ssl".to_string(), ConfigValue::from(false));
    /// defaults.insert("server.timeout".to_string(), ConfigValue::from(30i64));
    ///
    /// spice.set_defaults(defaults).unwrap();
    ///
    /// // All defaults are now available
    /// assert_eq!(spice.get_string("database.host").unwrap(), Some("localhost".to_string()));
    /// assert_eq!(spice.get_i64("database.port").unwrap(), Some(5432));
    /// ```
    pub fn set_defaults(&mut self, defaults: HashMap<String, ConfigValue>) -> ConfigResult<()> {
        // Find or create a default layer
        let default_layer_index = self
            .layers
            .iter()
            .position(|layer| layer.priority() == LayerPriority::Defaults);

        match default_layer_index {
            Some(index) => {
                // Update existing default layer
                let layer = &mut self.layers[index];
                for (key, value) in defaults {
                    layer.set(&key, value)?;
                }
            }
            None => {
                // Create new default layer with all defaults
                let default_layer = DefaultConfigLayer::with_defaults(defaults);
                self.add_layer(Box::new(default_layer));
            }
        }

        Ok(())
    }

    /// Gets a configuration value as a string.
    ///
    /// # Arguments
    /// * `key` - The configuration key to retrieve
    ///
    /// # Returns
    /// * `ConfigResult<Option<String>>` - The string value if found and convertible
    pub fn get_string(&mut self, key: &str) -> ConfigResult<Option<String>> {
        self.check_and_reload()?;
        match self.get(key)? {
            Some(value) => Ok(Some(value.coerce_to_string())),
            None => Ok(None),
        }
    }

    /// Gets a configuration value as an integer.
    ///
    /// # Arguments
    /// * `key` - The configuration key to retrieve
    ///
    /// # Returns
    /// * `ConfigResult<Option<i64>>` - The integer value if found and convertible
    pub fn get_int(&mut self, key: &str) -> ConfigResult<Option<i64>> {
        self.check_and_reload()?;
        match self.get(key)? {
            Some(value) => match value.as_i64() {
                Some(i) => Ok(Some(i)),
                None => Err(ConfigError::type_conversion(value.type_name(), "integer")),
            },
            None => Ok(None),
        }
    }

    /// Gets a configuration value as a 64-bit integer.
    ///
    /// # Arguments
    /// * `key` - The configuration key to retrieve
    ///
    /// # Returns
    /// * `ConfigResult<Option<i64>>` - The i64 value if found and convertible
    pub fn get_i64(&mut self, key: &str) -> ConfigResult<Option<i64>> {
        self.get_int(key)
    }

    /// Gets a configuration value as a 32-bit integer.
    ///
    /// # Arguments
    /// * `key` - The configuration key to retrieve
    ///
    /// # Returns
    /// * `ConfigResult<Option<i32>>` - The i32 value if found and convertible
    pub fn get_i32(&mut self, key: &str) -> ConfigResult<Option<i32>> {
        match self.get_int(key)? {
            Some(i) => {
                if i >= i32::MIN as i64 && i <= i32::MAX as i64 {
                    Ok(Some(i as i32))
                } else {
                    Err(ConfigError::type_conversion("i64", "i32"))
                }
            }
            None => Ok(None),
        }
    }

    /// Gets a configuration value as a floating point number.
    ///
    /// # Arguments
    /// * `key` - The configuration key to retrieve
    ///
    /// # Returns
    /// * `ConfigResult<Option<f64>>` - The float value if found and convertible
    pub fn get_float(&self, key: &str) -> ConfigResult<Option<f64>> {
        match self.get(key)? {
            Some(value) => match value.as_f64() {
                Some(f) => Ok(Some(f)),
                None => Err(ConfigError::type_conversion(value.type_name(), "float")),
            },
            None => Ok(None),
        }
    }

    /// Gets a configuration value as a 64-bit floating point number.
    ///
    /// # Arguments
    /// * `key` - The configuration key to retrieve
    ///
    /// # Returns
    /// * `ConfigResult<Option<f64>>` - The f64 value if found and convertible
    pub fn get_f64(&self, key: &str) -> ConfigResult<Option<f64>> {
        self.get_float(key)
    }

    /// Gets a configuration value as a 32-bit floating point number.
    ///
    /// # Arguments
    /// * `key` - The configuration key to retrieve
    ///
    /// # Returns
    /// * `ConfigResult<Option<f32>>` - The f32 value if found and convertible
    pub fn get_f32(&self, key: &str) -> ConfigResult<Option<f32>> {
        match self.get_float(key)? {
            Some(f) => {
                if f.is_finite() && f >= f32::MIN as f64 && f <= f32::MAX as f64 {
                    Ok(Some(f as f32))
                } else {
                    Err(ConfigError::type_conversion("f64", "f32"))
                }
            }
            None => Ok(None),
        }
    }

    /// Gets a configuration value as a boolean.
    ///
    /// # Arguments
    /// * `key` - The configuration key to retrieve
    ///
    /// # Returns
    /// * `ConfigResult<Option<bool>>` - The boolean value if found and convertible
    pub fn get_bool(&mut self, key: &str) -> ConfigResult<Option<bool>> {
        self.check_and_reload()?;
        match self.get(key)? {
            Some(value) => match value.coerce_to_bool() {
                Some(b) => Ok(Some(b)),
                None => Err(ConfigError::type_conversion(value.type_name(), "boolean")),
            },
            None => Ok(None),
        }
    }

    /// Gets a configuration value as an array.
    ///
    /// # Arguments
    /// * `key` - The configuration key to retrieve
    ///
    /// # Returns
    /// * `ConfigResult<Option<Vec<ConfigValue>>>` - The array value if found and convertible
    pub fn get_array(&self, key: &str) -> ConfigResult<Option<Vec<ConfigValue>>> {
        match self.get(key)? {
            Some(value) => match value.as_array() {
                Some(arr) => Ok(Some(arr.clone())),
                None => Err(ConfigError::type_conversion(value.type_name(), "array")),
            },
            None => Ok(None),
        }
    }

    /// Gets a configuration value as an object/map.
    ///
    /// # Arguments
    /// * `key` - The configuration key to retrieve
    ///
    /// # Returns
    /// * `ConfigResult<Option<HashMap<String, ConfigValue>>>` - The object value if found and convertible
    pub fn get_object(
        &self,
        key: &str,
    ) -> ConfigResult<Option<std::collections::HashMap<String, ConfigValue>>> {
        match self.get(key)? {
            Some(value) => match value.as_object() {
                Some(obj) => Ok(Some(obj.clone())),
                None => Err(ConfigError::type_conversion(value.type_name(), "object")),
            },
            None => Ok(None),
        }
    }

    /// Checks if a configuration key exists in any layer.
    ///
    /// # Arguments
    /// * `key` - The configuration key to check
    ///
    /// # Returns
    /// * `bool` - True if the key exists, false otherwise
    pub fn is_set(&self, key: &str) -> bool {
        self.get(key).unwrap_or(None).is_some()
    }

    /// Gets all configuration keys from all layers.
    ///
    /// # Returns
    /// * `Vec<String>` - All unique configuration keys
    pub fn all_keys(&self) -> Vec<String> {
        utils::collect_all_keys(&self.layers)
    }

    /// Creates a nested configuration structure from flat keys.
    /// This method takes a flat map of keys (like "database.host") and converts them
    /// into a nested structure suitable for serialization.
    ///
    /// # Arguments
    /// * `flat_settings` - A flat map of configuration keys and values
    ///
    /// # Returns
    /// * `HashMap<String, ConfigValue>` - A nested configuration structure
    ///
    /// This is an internal method used by serialization functions.
    fn expand_nested_keys(
        &self,
        flat_settings: HashMap<String, ConfigValue>,
    ) -> HashMap<String, ConfigValue> {
        let mut result = HashMap::new();

        // Sort keys by length (ascending) and then alphabetically
        // This ensures shorter (less specific) keys are processed first,
        // allowing longer (more specific) keys to overwrite them
        let mut sorted_keys: Vec<_> = flat_settings.keys().collect();
        sorted_keys.sort_by(|a, b| {
            a.len().cmp(&b.len()).then(a.cmp(b))
        });

        for key in sorted_keys {
            let value = flat_settings.get(key).unwrap();
            self.insert_nested_value(&mut result, key, value.clone());
        }

        result
    }

    /// Inserts a value into a nested structure using dot notation.
    ///
    /// # Arguments
    /// * `target` - The target map to insert into
    /// * `key` - The dot-separated key path
    /// * `value` - The value to insert
    fn insert_nested_value(
        &self,
        target: &mut HashMap<String, ConfigValue>,
        key: &str,
        value: ConfigValue,
    ) {
        let parts: Vec<&str> = key.split(&self.key_delimiter).collect();

        if parts.len() == 1 {
            // Simple key, insert directly
            target.insert(key.to_string(), value);
            return;
        }

        // Recursively create nested structure
        self.insert_nested_value_recursive(target, &parts, 0, value);
    }

    fn insert_nested_value_recursive(
        &self,
        current: &mut HashMap<String, ConfigValue>,
        parts: &[&str],
        index: usize,
        value: ConfigValue,
    ) {
        if index >= parts.len() {
            return;
        }

        let part = parts[index];

        if index == parts.len() - 1 {
            // Last part, insert the value (always overwrite)
            current.insert(part.to_string(), value);
        } else {
            // Intermediate part, ensure we have an object
            let entry = current
                .entry(part.to_string())
                .or_insert_with(|| ConfigValue::Object(HashMap::new()));

            match entry {
                ConfigValue::Object(ref mut obj) => {
                    self.insert_nested_value_recursive(obj, parts, index + 1, value);
                }
                _ => {
                    // Overwrite non-object with object
                    *entry = ConfigValue::Object(HashMap::new());
                    if let ConfigValue::Object(ref mut obj) = entry {
                        self.insert_nested_value_recursive(obj, parts, index + 1, value);
                    }
                }
            }
        }
    }

    /// Gets all configuration settings as a merged map.
    ///
    /// # Returns
    /// * `ConfigResult<HashMap<String, ConfigValue>>` - All configuration settings merged by precedence
    pub fn all_settings(&self) -> ConfigResult<HashMap<String, ConfigValue>> {
        let flat_settings = utils::merge_all_layers(&self.layers)?;
        Ok(self.expand_nested_keys(flat_settings))
    }

    /// Gets all configuration settings optimized for serialization.
    /// This method performs enhanced merging and handles complex nested structures
    /// to ensure proper serialization to various formats.
    ///
    /// # Returns
    /// * `ConfigResult<HashMap<String, ConfigValue>>` - All configuration settings optimized for serialization
    pub fn all_settings_for_serialization(&self) -> ConfigResult<HashMap<String, ConfigValue>> {
        // Get flat settings from all layers with proper precedence
        let flat_settings = utils::merge_all_layers(&self.layers)?;

        // Expand nested keys and handle format-specific considerations
        let mut expanded = self.expand_nested_keys(flat_settings);

        // Perform additional processing for serialization compatibility
        self.optimize_for_serialization(&mut expanded);

        Ok(expanded)
    }

    /// Optimizes configuration data for serialization by handling edge cases
    /// and ensuring compatibility with different output formats.
    fn optimize_for_serialization(&self, settings: &mut HashMap<String, ConfigValue>) {
        // Recursively process all values
        for (_, value) in settings.iter_mut() {
            self.optimize_config_value_for_serialization(value);
        }
    }

    /// Recursively optimizes a ConfigValue for serialization.
    fn optimize_config_value_for_serialization(&self, value: &mut ConfigValue) {
        match value {
            ConfigValue::Object(obj) => {
                // Recursively optimize nested objects
                for (_, nested_value) in obj.iter_mut() {
                    self.optimize_config_value_for_serialization(nested_value);
                }
            }
            ConfigValue::Array(arr) => {
                // Recursively optimize array elements
                for element in arr.iter_mut() {
                    self.optimize_config_value_for_serialization(element);
                }
            }
            ConfigValue::Float(f) => {
                // Handle special float values that might not serialize well
                if f.is_nan() || f.is_infinite() {
                    *value = ConfigValue::String(f.to_string());
                }
            }
            _ => {
                // Other types are fine as-is
            }
        }
    }

    /// Writes the current configuration to a file.
    /// The file format is determined by the file extension.
    ///
    /// # Arguments
    /// * `filename` - The path to the file to write
    ///
    /// # Returns
    /// * `ConfigResult<()>` - Success if the file was written, or an error
    ///
    /// # Errors
    /// * `ConfigError::UnsupportedFormat` - If the file extension is not supported
    /// * `ConfigError::Io` - If the file cannot be written
    /// * `ConfigError::Serialization` - If the configuration cannot be serialized
    ///
    /// # Example
    /// ```no_run
    /// use spicex::Spice;
    ///
    /// let mut spice = Spice::new();
    /// spice.set("app.name", "my-app".into()).unwrap();
    /// spice.set("app.port", 8080i64.into()).unwrap();
    ///
    /// // Write to JSON file
    /// spice.write_config("config.json").unwrap();
    /// ```
    pub fn write_config<P: AsRef<Path>>(&self, filename: P) -> ConfigResult<()> {
        let path = filename.as_ref();

        // Get file extension to determine format
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or(ConfigError::UnsupportedFormat)?;

        // Get all current settings with enhanced merging
        let settings = self.all_settings_for_serialization()?;

        // Get the appropriate parser and serialize with enhanced error handling
        let parser = crate::parser::detect_parser_by_extension(extension).map_err(|e| {
            ConfigError::Serialization(format!(
                "Failed to detect parser for extension '{extension}': {e}"
            ))
        })?;

        let content = parser.serialize(&settings).map_err(|e| {
            ConfigError::Serialization(format!(
                "Failed to serialize configuration to {}: {}",
                extension.to_uppercase(),
                e
            ))
        })?;

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ConfigError::Io(std::io::Error::new(
                    e.kind(),
                    format!(
                        "Failed to create parent directories for '{}': {}",
                        path.display(),
                        e
                    ),
                ))
            })?;
        }

        // Write to file with enhanced error handling
        std::fs::write(path, content).map_err(|e| {
            ConfigError::Io(std::io::Error::new(
                e.kind(),
                format!(
                    "Failed to write configuration to '{}': {}",
                    path.display(),
                    e
                ),
            ))
        })?;

        Ok(())
    }

    /// Writes the current configuration to a file in a specific format.
    /// This method allows you to specify the format explicitly, regardless of file extension.
    ///
    /// # Arguments
    /// * `filename` - The path to the file to write
    /// * `format` - The format to use for serialization ("json", "yaml", "toml", "ini")
    ///
    /// # Returns
    /// * `ConfigResult<()>` - Success if the file was written, or an error
    ///
    /// # Errors
    /// * `ConfigError::UnsupportedFormat` - If the format is not supported
    /// * `ConfigError::Io` - If the file cannot be written
    /// * `ConfigError::Serialization` - If the configuration cannot be serialized
    ///
    /// # Example
    /// ```no_run
    /// use spicex::Spice;
    ///
    /// let mut spice = Spice::new();
    /// spice.set("app.name", "my-app".into()).unwrap();
    /// spice.set("app.port", 8080i64.into()).unwrap();
    ///
    /// // Write as YAML regardless of file extension
    /// spice.write_config_as("config.txt", "yaml").unwrap();
    /// ```
    pub fn write_config_as<P: AsRef<Path>>(&self, filename: P, format: &str) -> ConfigResult<()> {
        let path = filename.as_ref();

        // Get all current settings with enhanced merging and serialization optimization
        let settings = self.all_settings_for_serialization()?;

        // Get the appropriate parser and serialize with enhanced error handling
        let parser = crate::parser::detect_parser_by_extension(format).map_err(|e| {
            ConfigError::Serialization(format!(
                "Failed to detect parser for format '{format}': {e}"
            ))
        })?;

        let content = parser.serialize(&settings).map_err(|e| {
            ConfigError::Serialization(format!(
                "Failed to serialize configuration to {}: {}",
                format.to_uppercase(),
                e
            ))
        })?;

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    ConfigError::Io(std::io::Error::new(
                        e.kind(),
                        format!(
                            "Failed to create parent directories for '{}': {}",
                            path.display(),
                            e
                        ),
                    ))
                })?;
            }
        }

        // Write to file with enhanced error handling
        std::fs::write(path, content).map_err(|e| {
            ConfigError::Io(std::io::Error::new(
                e.kind(),
                format!(
                    "Failed to write configuration to '{}': {}",
                    path.display(),
                    e
                ),
            ))
        })?;

        Ok(())
    }

    /// Safely writes the current configuration to a file, preventing overwriting existing files.
    /// This method will fail if the target file already exists.
    ///
    /// # Arguments
    /// * `filename` - The path to the file to write
    ///
    /// # Returns
    /// * `ConfigResult<()>` - Success if the file was written, or an error
    ///
    /// # Errors
    /// * `ConfigError::Io` - If the file already exists or cannot be written
    /// * `ConfigError::UnsupportedFormat` - If the file extension is not supported
    /// * `ConfigError::Serialization` - If the configuration cannot be serialized
    ///
    /// # Example
    /// ```no_run
    /// use spicex::Spice;
    ///
    /// let mut spice = Spice::new();
    /// spice.set("app.name", "my-app".into()).unwrap();
    ///
    /// // This will fail if config.json already exists
    /// match spice.safe_write_config("config.json") {
    ///     Ok(()) => println!("Configuration written successfully"),
    ///     Err(e) => println!("Failed to write config: {}", e),
    /// }
    /// ```
    pub fn safe_write_config<P: AsRef<Path>>(&self, filename: P) -> ConfigResult<()> {
        let path = filename.as_ref();

        // Check if file already exists
        if path.exists() {
            return Err(ConfigError::Io(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("File '{}' already exists", path.display()),
            )));
        }

        // Use regular write_config if file doesn't exist
        self.write_config(path)
    }

    /// Creates a sub-configuration focused on a specific key prefix.
    /// This allows working with a subsection of the configuration as if it were the root.
    ///
    /// # Arguments
    /// * `key` - The key prefix to focus on (e.g., "database" to work with database.* keys)
    ///
    /// # Returns
    /// * `ConfigResult<Option<Spice>>` - A new Spice instance focused on the subsection, or None if the key doesn't exist
    ///
    /// # Example
    /// ```
    /// use spicex::{Spice, ConfigValue};
    /// use std::collections::HashMap;
    ///
    /// let mut spice = Spice::new();
    /// let mut db_config = HashMap::new();
    /// db_config.insert("host".to_string(), ConfigValue::from("localhost"));
    /// db_config.insert("port".to_string(), ConfigValue::from(5432i64));
    /// spice.set("database", ConfigValue::Object(db_config)).unwrap();
    ///
    /// // Create a sub-configuration for database settings
    /// if let Some(db_viper) = spice.sub("database").unwrap() {
    ///     // Now you can access "host" directly instead of "database.host"
    ///     let host = db_viper.get_string("host").unwrap();
    ///     assert_eq!(host, Some("localhost".to_string()));
    /// }
    /// ```
    pub fn sub(&self, key: &str) -> ConfigResult<Option<Spice>> {
        // Get the value at the specified key
        match self.get(key)? {
            Some(ConfigValue::Object(obj)) => {
                // Create a new Spice instance with the object data
                let mut sub_viper = Spice::new();
                sub_viper.key_delimiter = self.key_delimiter.clone();

                // Create a sub-configuration layer with the object data
                let sub_layer = SubConfigLayer::new(key, obj);
                sub_viper.add_layer(Box::new(sub_layer));

                Ok(Some(sub_viper))
            }
            Some(_) => {
                // The key exists but is not an object, so we can't create a sub-configuration
                Ok(None)
            }
            None => {
                // The key doesn't exist
                Ok(None)
            }
        }
    }

    /// Unmarshals the entire configuration into a struct that implements Deserialize.
    /// This method uses serde to deserialize the merged configuration from all layers
    /// into the target struct type.
    ///
    /// # Type Parameters
    /// * `T` - The target struct type that implements serde::Deserialize
    ///
    /// # Returns
    /// * `ConfigResult<T>` - The deserialized struct or an error if deserialization fails
    ///
    /// # Example
    /// ```
    /// use spicex::{Spice, ConfigValue};
    /// use serde::Deserialize;
    /// use std::collections::HashMap;
    ///
    /// #[derive(Deserialize, Debug, PartialEq)]
    /// struct DatabaseConfig {
    ///     host: String,
    ///     port: u16,
    ///     #[serde(default)]
    ///     ssl: bool,
    /// }
    ///
    /// #[derive(Deserialize, Debug, PartialEq)]
    /// struct AppConfig {
    ///     database: DatabaseConfig,
    ///     debug: bool,
    /// }
    ///
    /// let mut spice = Spice::new();
    /// let mut db_config = HashMap::new();
    /// db_config.insert("host".to_string(), ConfigValue::from("localhost"));
    /// db_config.insert("port".to_string(), ConfigValue::from(5432i64));
    /// spice.set("database", ConfigValue::Object(db_config)).unwrap();
    /// spice.set("debug", ConfigValue::from(true)).unwrap();
    ///
    /// let config: AppConfig = spice.unmarshal().unwrap();
    /// assert_eq!(config.database.host, "localhost");
    /// assert_eq!(config.database.port, 5432);
    /// assert_eq!(config.debug, true);
    /// ```
    pub fn unmarshal<T>(&self) -> ConfigResult<T>
    where
        T: serde::de::DeserializeOwned,
    {
        // Get all settings merged from all layers
        let all_settings = self.all_settings()?;

        // Convert the HashMap<String, ConfigValue> to a ConfigValue::Object
        let config_value = ConfigValue::Object(all_settings);

        // Use serde to deserialize the ConfigValue into the target type
        serde_json::from_value(serde_json::to_value(config_value)?).map_err(|e| {
            ConfigError::deserialization(format!("Failed to unmarshal configuration: {e}"))
        })
    }

    /// Unmarshals a specific configuration key into a struct that implements Deserialize.
    /// This method allows deserializing only a portion of the configuration.
    ///
    /// # Arguments
    /// * `key` - The configuration key to unmarshal (supports dot notation for nested access)
    ///
    /// # Type Parameters
    /// * `T` - The target struct type that implements serde::Deserialize
    ///
    /// # Returns
    /// * `ConfigResult<T>` - The deserialized struct or an error if the key doesn't exist or deserialization fails
    ///
    /// # Example
    /// ```
    /// use spicex::{Spice, ConfigValue};
    /// use serde::Deserialize;
    /// use std::collections::HashMap;
    ///
    /// #[derive(Deserialize, Debug, PartialEq)]
    /// struct DatabaseConfig {
    ///     host: String,
    ///     port: u16,
    ///     #[serde(default)]
    ///     ssl: bool,
    /// }
    ///
    /// let mut spice = Spice::new();
    /// let mut db_config = HashMap::new();
    /// db_config.insert("host".to_string(), ConfigValue::from("localhost"));
    /// db_config.insert("port".to_string(), ConfigValue::from(5432i64));
    /// spice.set("database", ConfigValue::Object(db_config)).unwrap();
    ///
    /// let db_config: DatabaseConfig = spice.unmarshal_key("database").unwrap();
    /// assert_eq!(db_config.host, "localhost");
    /// assert_eq!(db_config.port, 5432);
    /// assert_eq!(db_config.ssl, false); // default value
    /// ```
    pub fn unmarshal_key<T>(&self, key: &str) -> ConfigResult<T>
    where
        T: serde::de::DeserializeOwned,
    {
        // Get the value at the specified key
        let config_value = self
            .get(key)?
            .ok_or_else(|| ConfigError::key_not_found(key))?;

        // Use serde to deserialize the ConfigValue into the target type
        serde_json::from_value(serde_json::to_value(config_value)?).map_err(|e| {
            ConfigError::deserialization(format!("Failed to unmarshal key '{key}': {e}"))
        })
    }

    /// Unmarshals the entire configuration into a struct with validation.
    /// This method deserializes the configuration and then validates it using the provided validator function.
    ///
    /// # Arguments
    /// * `validator` - A function that validates the deserialized struct and returns a Result
    ///
    /// # Type Parameters
    /// * `T` - The target struct type that implements serde::Deserialize
    ///
    /// # Returns
    /// * `ConfigResult<T>` - The validated deserialized struct or an error if deserialization or validation fails
    ///
    /// # Example
    /// ```
    /// use spicex::{Spice, ConfigValue, ConfigError};
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize, Debug, PartialEq)]
    /// struct ServerConfig {
    ///     host: String,
    ///     port: u16,
    /// }
    ///
    /// impl ServerConfig {
    ///     fn validate(&self) -> Result<(), String> {
    ///         if self.port == 0 {
    ///             return Err("Port cannot be zero".to_string());
    ///         }
    ///         if self.host.is_empty() {
    ///             return Err("Host cannot be empty".to_string());
    ///         }
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let mut spice = Spice::new();
    /// spice.set("host", ConfigValue::from("localhost")).unwrap();
    /// spice.set("port", ConfigValue::from(8080i64)).unwrap();
    ///
    /// let config: ServerConfig = spice.unmarshal_with_validation(|config: &ServerConfig| {
    ///     config.validate().map_err(|e| ConfigError::invalid_value(e))
    /// }).unwrap();
    /// ```
    pub fn unmarshal_with_validation<T, F>(&self, validator: F) -> ConfigResult<T>
    where
        T: serde::de::DeserializeOwned,
        F: FnOnce(&T) -> ConfigResult<()>,
    {
        let config: T = self.unmarshal()?;
        validator(&config)?;
        Ok(config)
    }

    /// Unmarshals a specific configuration key into a struct with validation.
    /// This method deserializes a specific configuration section and then validates it.
    ///
    /// # Arguments
    /// * `key` - The configuration key to unmarshal (supports dot notation for nested access)
    /// * `validator` - A function that validates the deserialized struct and returns a Result
    ///
    /// # Type Parameters
    /// * `T` - The target struct type that implements serde::Deserialize
    ///
    /// # Returns
    /// * `ConfigResult<T>` - The validated deserialized struct or an error if deserialization or validation fails
    ///
    /// # Example
    /// ```
    /// use spicex::{Spice, ConfigValue, ConfigError};
    /// use serde::Deserialize;
    /// use std::collections::HashMap;
    ///
    /// #[derive(Deserialize, Debug, PartialEq)]
    /// struct DatabaseConfig {
    ///     host: String,
    ///     port: u16,
    /// }
    ///
    /// impl DatabaseConfig {
    ///     fn validate(&self) -> Result<(), String> {
    ///         if self.port < 1024 {
    ///             return Err("Port should be >= 1024 for non-privileged access".to_string());
    ///         }
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let mut spice = Spice::new();
    /// let mut db_config = HashMap::new();
    /// db_config.insert("host".to_string(), ConfigValue::from("localhost"));
    /// db_config.insert("port".to_string(), ConfigValue::from(5432i64));
    /// spice.set("database", ConfigValue::Object(db_config)).unwrap();
    ///
    /// let config: DatabaseConfig = spice.unmarshal_key_with_validation("database", |config: &DatabaseConfig| {
    ///     config.validate().map_err(|e| ConfigError::invalid_value(e))
    /// }).unwrap();
    /// ```
    pub fn unmarshal_key_with_validation<T, F>(&self, key: &str, validator: F) -> ConfigResult<T>
    where
        T: serde::de::DeserializeOwned,
        F: FnOnce(&T) -> ConfigResult<()>,
    {
        let config: T = self.unmarshal_key(key)?;
        validator(&config)?;
        Ok(config)
    }

    /// Enables automatic reloading of configuration files when they change.
    /// This method sets up file system watching for all currently loaded configuration files
    /// and will automatically reload them when changes are detected.
    ///
    /// # Returns
    /// * `ConfigResult<()>` - Success if file watching was enabled, or an error
    ///
    /// # Errors
    /// * `ConfigError::FileWatch` - If file watching cannot be initialized
    ///
    /// # Example
    /// ```no_run
    /// use spicex::Spice;
    ///
    /// let mut spice = Spice::new();
    /// spice.set_config_name("config");
    /// spice.read_in_config().unwrap();
    ///
    /// // Enable automatic reloading when config files change
    /// spice.watch_config().unwrap();
    ///
    /// // Configuration will now automatically reload when files change
    /// ```
    pub fn watch_config(&mut self) -> ConfigResult<()> {
        // Collect all file paths from FileConfigLayer instances
        let mut config_files = Vec::new();

        for layer in &self.layers {
            if let Some(file_layer) = layer.as_any().downcast_ref::<FileConfigLayer>() {
                config_files.push(file_layer.file_path().to_path_buf());
            }
        }

        if config_files.is_empty() {
            return Err(ConfigError::FileWatch(
                "No configuration files to watch. Load a configuration file first.".to_string(),
            ));
        }

        // Create file watcher if it doesn't exist
        if self.watcher.is_none() {
            self.watcher = Some(FileWatcher::new_empty()?);
        }

        let watcher = self.watcher.as_mut().unwrap();

        // Watch all configuration files
        for config_file in &config_files {
            if !watcher.watched_files().contains(config_file) {
                watcher.watch_file(config_file)?;
            }
        }

        // Store the list of watched files
        self.watched_config_files = config_files;

        // Start watching in background
        watcher.start_watching()?;

        Ok(())
    }

    /// Registers a callback to be called when configuration files change.
    /// This method allows you to register custom handlers that will be called
    /// whenever a watched configuration file is modified.
    ///
    /// # Arguments
    /// * `callback` - A function to call when configuration changes are detected
    ///
    /// # Returns
    /// * `ConfigResult<()>` - Success if the callback was registered, or an error
    ///
    /// # Errors
    /// * `ConfigError::FileWatch` - If file watching is not enabled or callback registration fails
    ///
    /// # Example
    /// ```no_run
    /// use spicex::Spice;
    /// use std::sync::{Arc, Mutex};
    ///
    /// let mut spice = Spice::new();
    /// spice.set_config_name("config");
    /// spice.read_in_config().unwrap();
    /// spice.watch_config().unwrap();
    ///
    /// let reload_count = Arc::new(Mutex::new(0));
    /// let reload_count_clone = Arc::clone(&reload_count);
    ///
    /// spice.on_config_change(move || {
    ///     let mut count = reload_count_clone.lock().unwrap();
    ///     *count += 1;
    ///     println!("Configuration reloaded {} times", *count);
    /// }).unwrap();
    /// ```
    pub fn on_config_change<F>(&mut self, callback: F) -> ConfigResult<()>
    where
        F: Fn() + Send + Sync + 'static,
    {
        if self.watcher.is_none() {
            return Err(ConfigError::FileWatch(
                "File watching is not enabled. Call watch_config() first.".to_string(),
            ));
        }

        // First register the automatic reload callback
        self.register_auto_reload_callback()?;

        // Store the user's callback to be triggered only after successful reloads
        self.user_callbacks.push(Box::new(callback));

        Ok(())
    }

    /// Registers an internal callback for automatic configuration reloading.
    /// This method sets up the automatic reloading functionality that refreshes
    /// configuration layers when file changes are detected.
    fn register_auto_reload_callback(&mut self) -> ConfigResult<()> {
        if self.auto_reload_registered {
            return Ok(()); // Already registered
        }

        // Clone the needs_reload flag for the callback
        let needs_reload = Arc::clone(&self.needs_reload);

        // Register a callback that sets the reload flag but doesn't trigger user callbacks yet
        if let Some(watcher) = &mut self.watcher {
            watcher.on_config_change(move || {
                needs_reload.store(true, std::sync::atomic::Ordering::SeqCst);
            })?;
        }

        self.auto_reload_registered = true;
        Ok(())
    }

    /// Checks if configuration needs to be reloaded and performs the reload if necessary.
    /// Returns true if a reload was actually performed, false otherwise.
    fn check_and_reload(&mut self) -> ConfigResult<bool> {
        if self.needs_reload.load(std::sync::atomic::Ordering::SeqCst) {
            // Try to reload, but first check if all files are still valid
            let reload_successful = self.try_reload_if_valid()?;
            if reload_successful {
                // Reset the reload flag only if reload was successful
                self.needs_reload.store(false, std::sync::atomic::Ordering::SeqCst);

                // Trigger all user callbacks after successful reload
                for callback in &self.user_callbacks {
                    callback();
                }

                return Ok(true);
            } else {
                // If reload failed (due to invalid files), reset flag but don't reload
                self.needs_reload.store(false, std::sync::atomic::Ordering::SeqCst);
                return Ok(false);
            }
        }
        Ok(false)
    }

    /// Attempts to reload configuration only if all watched files are valid.
    /// Returns true if reload was successful, false if any file was invalid.
    fn try_reload_if_valid(&mut self) -> ConfigResult<bool> {
        if self.watched_config_files.is_empty() {
            return Ok(false);
        }

        // First, validate all files can be parsed
        let mut new_file_layers = Vec::new();
        for config_file in &self.watched_config_files {
            match FileConfigLayer::new(config_file) {
                Ok(file_layer) => new_file_layers.push(file_layer),
                Err(_) => {
                    // If any file is invalid, don't reload
                    return Ok(false);
                }
            }
        }

        // Only if all files are valid, proceed with the reload
        // Remove existing file layers
        self.layers.retain(|layer| {
            layer.as_any().downcast_ref::<FileConfigLayer>().is_none()
        });

        // Add the new valid file layers
        for file_layer in new_file_layers {
            self.add_layer(Box::new(file_layer));
        }

        Ok(true)
    }

    /// Stops watching configuration files for changes.
    /// This method disables automatic reloading and stops the file watching background thread.
    ///
    /// # Example
    /// ```no_run
    /// use spicex::Spice;
    ///
    /// let mut spice = Spice::new();
    /// spice.set_config_name("config");
    /// spice.read_in_config().unwrap();
    /// spice.watch_config().unwrap();
    ///
    /// // Later, stop watching
    /// spice.stop_watching();
    /// ```
    pub fn stop_watching(&mut self) {
        if let Some(watcher) = &mut self.watcher {
            watcher.stop_watching();
        }
        self.watched_config_files.clear();
    }

    /// Returns whether configuration file watching is currently active.
    ///
    /// # Returns
    /// * `bool` - True if file watching is active, false otherwise
    ///
    /// # Example
    /// ```no_run
    /// use spicex::Spice;
    ///
    /// let mut spice = Spice::new();
    /// assert!(!spice.is_watching());
    ///
    /// spice.set_config_name("config");
    /// spice.read_in_config().unwrap();
    /// spice.watch_config().unwrap();
    /// assert!(spice.is_watching());
    /// ```
    pub fn is_watching(&self) -> bool {
        self.watcher.as_ref().is_some_and(|w| w.is_watching())
    }

    /// Returns the list of configuration files currently being watched.
    ///
    /// # Returns
    /// * `&[PathBuf]` - Slice of paths to watched configuration files
    ///
    /// # Example
    /// ```no_run
    /// use spicex::Spice;
    ///
    /// let mut spice = Spice::new();
    /// spice.set_config_name("config");
    /// spice.read_in_config().unwrap();
    /// spice.watch_config().unwrap();
    ///
    /// let watched_files = spice.watched_config_files();
    /// println!("Watching {} configuration files", watched_files.len());
    /// ```
    pub fn watched_config_files(&self) -> &[PathBuf] {
        &self.watched_config_files
    }

    /// Processes pending reload signals from file watchers.
    /// This method should be called periodically to handle automatic reloading.
    /// It's automatically called by other methods that access configuration values.
    ///
    /// # Returns
    /// * `ConfigResult<bool>` - True if configuration was reloaded, false if no reload was needed
    ///
    /// # Errors
    /// * `ConfigError::Io` - If configuration files cannot be read during reload
    /// * `ConfigError::Parse` - If configuration files cannot be parsed during reload
    pub fn process_reload_signals(&mut self) -> ConfigResult<bool> {
        if let Some(receiver) = &self.reload_receiver {
            // Check for reload signals without blocking
            match receiver.try_recv() {
                Ok(()) => {
                    // Reload signal received, refresh file layers
                    self.reload_file_layers()?;
                    Ok(true)
                }
                Err(mpsc::TryRecvError::Empty) => {
                    // No signals pending
                    Ok(false)
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    // Channel disconnected, disable auto-reload
                    self.reload_receiver = None;
                    self.auto_reload_registered = false;
                    Ok(false)
                }
            }
        } else {
            Ok(false)
        }
    }

    /// Reloads all file-based configuration layers.
    /// This method refreshes the content of all FileConfigLayer instances
    /// while preserving their position in the layer hierarchy.
    ///
    /// # Returns
    /// * `ConfigResult<()>` - Success if all layers were reloaded, or an error
    ///
    /// # Errors
    /// * `ConfigError::Io` - If any configuration file cannot be read
    /// * `ConfigError::Parse` - If any configuration file cannot be parsed
    fn reload_file_layers(&mut self) -> ConfigResult<()> {
        let mut reload_errors = Vec::new();

        // Reload each file layer
        for layer in &mut self.layers {
            if let Some(file_layer) = layer.as_any_mut().downcast_mut::<FileConfigLayer>() {
                if let Err(e) = file_layer.reload() {
                    // Collect errors but continue trying to reload other layers
                    reload_errors.push((file_layer.file_path().to_string_lossy().to_string(), e));
                }
            }
        }

        // If there were any errors, report the first one
        // In a production system, you might want to handle this differently
        if let Some((file_path, error)) = reload_errors.first() {
            return Err(ConfigError::FileWatch(format!(
                "Failed to reload configuration file '{file_path}': {error}"
            )));
        }

        Ok(())
    }
}

/// Explicit configuration layer for values set directly via set() method.
struct ExplicitConfigLayer {
    data: std::collections::HashMap<String, ConfigValue>,
}

impl ExplicitConfigLayer {
    fn new() -> Self {
        Self {
            data: std::collections::HashMap::new(),
        }
    }
}

impl ConfigLayer for ExplicitConfigLayer {
    fn get(&self, key: &str) -> ConfigResult<Option<ConfigValue>> {
        Ok(self.data.get(key).cloned())
    }

    fn set(&mut self, key: &str, value: ConfigValue) -> ConfigResult<()> {
        self.data.insert(key.to_string(), value);
        Ok(())
    }

    fn keys(&self) -> Vec<String> {
        self.data.keys().cloned().collect()
    }

    fn source_name(&self) -> &str {
        "explicit"
    }

    fn priority(&self) -> LayerPriority {
        LayerPriority::Explicit
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Sub-configuration layer for focused access to a configuration subsection.
struct SubConfigLayer {
    data: std::collections::HashMap<String, ConfigValue>,
    source_key: String,
}

impl SubConfigLayer {
    fn new(source_key: &str, obj: std::collections::HashMap<String, ConfigValue>) -> Self {
        Self {
            data: obj,
            source_key: source_key.to_string(),
        }
    }
}

impl ConfigLayer for SubConfigLayer {
    fn get(&self, key: &str) -> ConfigResult<Option<ConfigValue>> {
        Ok(self.data.get(key).cloned())
    }

    fn set(&mut self, key: &str, value: ConfigValue) -> ConfigResult<()> {
        self.data.insert(key.to_string(), value);
        Ok(())
    }

    fn keys(&self) -> Vec<String> {
        self.data.keys().cloned().collect()
    }

    fn source_name(&self) -> &str {
        &self.source_key
    }

    fn priority(&self) -> LayerPriority {
        LayerPriority::Explicit
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl Default for Spice {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // Mock implementation for testing
    struct MockConfigLayer {
        data: HashMap<String, ConfigValue>,
        priority: LayerPriority,
        name: String,
    }

    impl MockConfigLayer {
        fn new(name: &str, priority: LayerPriority) -> Self {
            Self {
                data: HashMap::new(),
                priority,
                name: name.to_string(),
            }
        }

        fn with_value(mut self, key: &str, value: ConfigValue) -> Self {
            self.data.insert(key.to_string(), value);
            self
        }
    }

    impl ConfigLayer for MockConfigLayer {
        fn get(&self, key: &str) -> ConfigResult<Option<ConfigValue>> {
            Ok(self.data.get(key).cloned())
        }

        fn set(&mut self, key: &str, value: ConfigValue) -> ConfigResult<()> {
            self.data.insert(key.to_string(), value);
            Ok(())
        }

        fn keys(&self) -> Vec<String> {
            self.data.keys().cloned().collect()
        }

        fn source_name(&self) -> &str {
            &self.name
        }

        fn priority(&self) -> LayerPriority {
            self.priority
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    #[test]
    fn test_new_viper() {
        let spice = Spice::new();
        assert_eq!(spice.layers.len(), 0);
        assert_eq!(spice.config_paths.len(), 0);
        assert_eq!(spice.key_delimiter, ".");
        assert!(!spice.automatic_env);
        assert_eq!(spice.config_name, "");
        assert!(spice.env_prefix.is_none());
    }

    #[test]
    fn test_default_viper() {
        let spice = Spice::default();
        assert_eq!(spice.layers.len(), 0);
        assert_eq!(spice.key_delimiter, ".");
    }

    #[test]
    fn test_add_layer() {
        let mut spice = Spice::new();
        assert_eq!(spice.layer_count(), 0);

        // Add a layer
        let layer = Box::new(MockConfigLayer::new("test", LayerPriority::ConfigFile));
        spice.add_layer(layer);
        assert_eq!(spice.layer_count(), 1);

        // Add another layer with higher priority
        let layer = Box::new(MockConfigLayer::new("env", LayerPriority::Environment));
        spice.add_layer(layer);
        assert_eq!(spice.layer_count(), 2);

        // Verify layers are sorted by priority
        let layer_info = spice.layer_info();
        assert_eq!(layer_info[0].1, LayerPriority::Environment); // Higher priority first
        assert_eq!(layer_info[1].1, LayerPriority::ConfigFile);
    }

    #[test]
    fn test_remove_layers_by_priority() {
        let mut spice = Spice::new();

        // Add multiple layers
        spice.add_layer(Box::new(MockConfigLayer::new(
            "config1",
            LayerPriority::ConfigFile,
        )));
        spice.add_layer(Box::new(MockConfigLayer::new(
            "config2",
            LayerPriority::ConfigFile,
        )));
        spice.add_layer(Box::new(MockConfigLayer::new(
            "env",
            LayerPriority::Environment,
        )));
        assert_eq!(spice.layer_count(), 3);

        // Remove config file layers
        let removed = spice.remove_layers_by_priority(LayerPriority::ConfigFile);
        assert_eq!(removed, 2);
        assert_eq!(spice.layer_count(), 1);

        // Verify only environment layer remains
        let layer_info = spice.layer_info();
        assert_eq!(layer_info.len(), 1);
        assert_eq!(layer_info[0].1, LayerPriority::Environment);
    }

    #[test]
    fn test_clear_layers() {
        let mut spice = Spice::new();
        spice.add_layer(Box::new(MockConfigLayer::new(
            "test",
            LayerPriority::ConfigFile,
        )));
        assert_eq!(spice.layer_count(), 1);

        spice.clear_layers();
        assert_eq!(spice.layer_count(), 0);
    }

    #[test]
    fn test_layer_info() {
        let mut spice = Spice::new();
        spice.add_layer(Box::new(MockConfigLayer::new(
            "config",
            LayerPriority::ConfigFile,
        )));
        spice.add_layer(Box::new(MockConfigLayer::new(
            "env",
            LayerPriority::Environment,
        )));

        let layer_info = spice.layer_info();
        assert_eq!(layer_info.len(), 2);

        // Should be sorted by priority
        assert_eq!(layer_info[0].0, "env");
        assert_eq!(layer_info[0].1, LayerPriority::Environment);
        assert_eq!(layer_info[1].0, "config");
        assert_eq!(layer_info[1].1, LayerPriority::ConfigFile);
    }

    #[test]
    fn test_config_name() {
        let mut spice = Spice::new();
        assert_eq!(spice.config_name(), "");

        spice.set_config_name("myapp");
        assert_eq!(spice.config_name(), "myapp");

        spice.set_config_name("another_name".to_string());
        assert_eq!(spice.config_name(), "another_name");
    }

    #[test]
    fn test_config_paths() {
        let mut spice = Spice::new();
        assert_eq!(spice.config_paths().len(), 0);

        spice.add_config_path("/etc/myapp");
        spice.add_config_path(PathBuf::from("/home/user/.config"));
        assert_eq!(spice.config_paths().len(), 2);
        assert_eq!(spice.config_paths()[0], PathBuf::from("/etc/myapp"));
        assert_eq!(spice.config_paths()[1], PathBuf::from("/home/user/.config"));
    }

    #[test]
    fn test_env_prefix() {
        let mut spice = Spice::new();
        assert!(spice.env_prefix().is_none());

        spice.set_env_prefix("MYAPP");
        assert_eq!(spice.env_prefix(), Some("MYAPP"));

        spice.set_env_prefix("ANOTHER".to_string());
        assert_eq!(spice.env_prefix(), Some("ANOTHER"));
    }

    #[test]
    fn test_automatic_env() {
        let mut spice = Spice::new();
        assert!(!spice.is_automatic_env());

        spice.set_automatic_env(true);
        assert!(spice.is_automatic_env());

        spice.set_automatic_env(false);
        assert!(!spice.is_automatic_env());
    }

    #[test]
    fn test_key_delimiter() {
        let mut spice = Spice::new();
        assert_eq!(spice.key_delimiter(), ".");

        spice.set_key_delimiter("_");
        assert_eq!(spice.key_delimiter(), "_");

        spice.set_key_delimiter("::".to_string());
        assert_eq!(spice.key_delimiter(), "::");
    }

    #[test]
    fn test_set_and_get() {
        let mut spice = Spice::new();

        // Test setting and getting a string value
        spice
            .set("test.key", ConfigValue::String("test_value".to_string()))
            .unwrap();
        let value = spice.get("test.key").unwrap();
        assert_eq!(value, Some(ConfigValue::String("test_value".to_string())));

        // Test getting non-existent key
        let value = spice.get("nonexistent.key").unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn test_explicit_layer_creation() {
        let mut spice = Spice::new();
        assert_eq!(spice.layer_count(), 0);

        // Setting a value should create an explicit layer
        spice
            .set("key1", ConfigValue::String("value1".to_string()))
            .unwrap();
        assert_eq!(spice.layer_count(), 1);

        // Setting another value should reuse the explicit layer
        spice
            .set("key2", ConfigValue::String("value2".to_string()))
            .unwrap();
        assert_eq!(spice.layer_count(), 1);

        // Verify the layer has explicit priority
        let layer_info = spice.layer_info();
        assert_eq!(layer_info[0].1, LayerPriority::Explicit);
    }

    #[test]
    fn test_precedence_with_set() {
        let mut spice = Spice::new();

        // Add a lower priority layer
        let layer = Box::new(
            MockConfigLayer::new("config", LayerPriority::ConfigFile).with_value(
                "shared_key",
                ConfigValue::String("config_value".to_string()),
            ),
        );
        spice.add_layer(layer);

        // Explicit set should override
        spice
            .set(
                "shared_key",
                ConfigValue::String("explicit_value".to_string()),
            )
            .unwrap();

        let value = spice.get("shared_key").unwrap();
        assert_eq!(
            value,
            Some(ConfigValue::String("explicit_value".to_string()))
        );
    }

    #[test]
    fn test_unmarshal_full_config() {
        use serde::Deserialize;

        #[derive(Deserialize, Debug, PartialEq)]
        struct TestConfig {
            name: String,
            port: u16,
            debug: bool,
        }

        let mut spice = Spice::new();
        spice.set("name", ConfigValue::from("test_app")).unwrap();
        spice.set("port", ConfigValue::from(8080i64)).unwrap();
        spice.set("debug", ConfigValue::from(true)).unwrap();

        let config: TestConfig = spice.unmarshal().unwrap();
        assert_eq!(config.name, "test_app");
        assert_eq!(config.port, 8080);
        assert_eq!(config.debug, true);
    }

    #[test]
    fn test_unmarshal_nested_config() {
        use serde::Deserialize;

        #[derive(Deserialize, Debug, PartialEq)]
        struct DatabaseConfig {
            host: String,
            port: u16,
        }

        #[derive(Deserialize, Debug, PartialEq)]
        struct AppConfig {
            database: DatabaseConfig,
            debug: bool,
        }

        let mut spice = Spice::new();

        // Set up nested database configuration
        let mut db_config = HashMap::new();
        db_config.insert("host".to_string(), ConfigValue::from("localhost"));
        db_config.insert("port".to_string(), ConfigValue::from(5432i64));
        spice
            .set("database", ConfigValue::Object(db_config))
            .unwrap();
        spice.set("debug", ConfigValue::from(false)).unwrap();

        let config: AppConfig = spice.unmarshal().unwrap();
        assert_eq!(config.database.host, "localhost");
        assert_eq!(config.database.port, 5432);
        assert_eq!(config.debug, false);
    }

    #[test]
    fn test_unmarshal_with_defaults() {
        use serde::Deserialize;

        #[derive(Deserialize, Debug, PartialEq)]
        struct ConfigWithDefaults {
            name: String,
            #[serde(default)]
            port: u16,
            #[serde(default = "default_debug")]
            debug: bool,
        }

        fn default_debug() -> bool {
            true
        }

        let mut spice = Spice::new();
        spice.set("name", ConfigValue::from("test_app")).unwrap();
        // Note: port and debug are not set, should use defaults

        let config: ConfigWithDefaults = spice.unmarshal().unwrap();
        assert_eq!(config.name, "test_app");
        assert_eq!(config.port, 0); // Default for u16
        assert_eq!(config.debug, true); // Custom default
    }

    #[test]
    fn test_unmarshal_key_specific() {
        use serde::Deserialize;

        #[derive(Deserialize, Debug, PartialEq)]
        struct DatabaseConfig {
            host: String,
            port: u16,
            #[serde(default)]
            ssl: bool,
        }

        let mut spice = Spice::new();

        // Set up database configuration
        let mut db_config = HashMap::new();
        db_config.insert("host".to_string(), ConfigValue::from("localhost"));
        db_config.insert("port".to_string(), ConfigValue::from(5432i64));
        spice
            .set("database", ConfigValue::Object(db_config))
            .unwrap();
        spice
            .set("other_key", ConfigValue::from("other_value"))
            .unwrap();

        // Unmarshal only the database section
        let db_config: DatabaseConfig = spice.unmarshal_key("database").unwrap();
        assert_eq!(db_config.host, "localhost");
        assert_eq!(db_config.port, 5432);
        assert_eq!(db_config.ssl, false); // Default value
    }

    #[test]
    fn test_unmarshal_key_missing() {
        use serde::Deserialize;

        #[derive(Deserialize, Debug, PartialEq)]
        struct TestConfig {
            name: String,
        }

        let spice = Spice::new();

        // Try to unmarshal a key that doesn't exist
        let result: Result<TestConfig, _> = spice.unmarshal_key("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().is_key_not_found());
    }

    #[test]
    fn test_unmarshal_type_mismatch() {
        use serde::Deserialize;

        #[derive(Deserialize, Debug, PartialEq)]
        struct TestConfig {
            port: u16,
        }

        let mut spice = Spice::new();
        // Set port as a string instead of number
        spice
            .set("port", ConfigValue::from("not_a_number"))
            .unwrap();

        // This should fail during deserialization
        let result: Result<TestConfig, _> = spice.unmarshal();
        assert!(result.is_err());
    }

    #[test]
    fn test_unmarshal_with_field_renaming() {
        use serde::Deserialize;

        #[derive(Deserialize, Debug, PartialEq)]
        struct TestConfig {
            #[serde(rename = "app_name")]
            name: String,
            #[serde(rename = "server_port")]
            port: u16,
        }

        let mut spice = Spice::new();
        spice.set("app_name", ConfigValue::from("my_app")).unwrap();
        spice
            .set("server_port", ConfigValue::from(3000i64))
            .unwrap();

        let config: TestConfig = spice.unmarshal().unwrap();
        assert_eq!(config.name, "my_app");
        assert_eq!(config.port, 3000);
    }

    #[test]
    fn test_unmarshal_array_config() {
        use serde::Deserialize;

        #[derive(Deserialize, Debug, PartialEq)]
        struct ServerConfig {
            host: String,
            port: u16,
        }

        #[derive(Deserialize, Debug, PartialEq)]
        struct AppConfig {
            servers: Vec<ServerConfig>,
        }

        let mut spice = Spice::new();

        // Create array of server configurations
        let servers = vec![
            ConfigValue::Object({
                let mut server1 = HashMap::new();
                server1.insert("host".to_string(), ConfigValue::from("server1.com"));
                server1.insert("port".to_string(), ConfigValue::from(8080i64));
                server1
            }),
            ConfigValue::Object({
                let mut server2 = HashMap::new();
                server2.insert("host".to_string(), ConfigValue::from("server2.com"));
                server2.insert("port".to_string(), ConfigValue::from(8081i64));
                server2
            }),
        ];

        spice.set("servers", ConfigValue::Array(servers)).unwrap();

        let config: AppConfig = spice.unmarshal().unwrap();
        assert_eq!(config.servers.len(), 2);
        assert_eq!(config.servers[0].host, "server1.com");
        assert_eq!(config.servers[0].port, 8080);
        assert_eq!(config.servers[1].host, "server2.com");
        assert_eq!(config.servers[1].port, 8081);
    }

    #[test]
    fn test_unmarshal_with_validation_success() {
        use serde::Deserialize;

        #[derive(Deserialize, Debug, PartialEq)]
        struct ServerConfig {
            host: String,
            port: u16,
        }

        impl ServerConfig {
            fn validate(&self) -> Result<(), String> {
                if self.port == 0 {
                    return Err("Port cannot be zero".to_string());
                }
                if self.host.is_empty() {
                    return Err("Host cannot be empty".to_string());
                }
                Ok(())
            }
        }

        let mut spice = Spice::new();
        spice.set("host", ConfigValue::from("localhost")).unwrap();
        spice.set("port", ConfigValue::from(8080i64)).unwrap();

        let config: ServerConfig = spice
            .unmarshal_with_validation(|config: &ServerConfig| {
                config.validate().map_err(|e| ConfigError::invalid_value(e))
            })
            .unwrap();

        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 8080);
    }

    #[test]
    fn test_unmarshal_with_validation_failure() {
        use serde::Deserialize;

        #[derive(Deserialize, Debug, PartialEq)]
        struct ServerConfig {
            host: String,
            port: u16,
        }

        impl ServerConfig {
            fn validate(&self) -> Result<(), String> {
                if self.port == 0 {
                    return Err("Port cannot be zero".to_string());
                }
                if self.host.is_empty() {
                    return Err("Host cannot be empty".to_string());
                }
                Ok(())
            }
        }

        let mut spice = Spice::new();
        spice.set("host", ConfigValue::from("")).unwrap(); // Invalid empty host
        spice.set("port", ConfigValue::from(8080i64)).unwrap();

        let result: Result<ServerConfig, _> =
            spice.unmarshal_with_validation(|config: &ServerConfig| {
                config.validate().map_err(|e| ConfigError::invalid_value(e))
            });

        assert!(result.is_err());
        if let Err(ConfigError::InvalidValue(msg)) = result {
            assert_eq!(msg, "Host cannot be empty");
        } else {
            panic!("Expected InvalidValue error");
        }
    }

    #[test]
    fn test_unmarshal_key_with_validation_success() {
        use serde::Deserialize;

        #[derive(Deserialize, Debug, PartialEq)]
        struct DatabaseConfig {
            host: String,
            port: u16,
        }

        impl DatabaseConfig {
            fn validate(&self) -> Result<(), String> {
                if self.port < 1024 {
                    return Err("Port should be >= 1024 for non-privileged access".to_string());
                }
                Ok(())
            }
        }

        let mut spice = Spice::new();
        let mut db_config = HashMap::new();
        db_config.insert("host".to_string(), ConfigValue::from("localhost"));
        db_config.insert("port".to_string(), ConfigValue::from(5432i64));
        spice
            .set("database", ConfigValue::Object(db_config))
            .unwrap();

        let config: DatabaseConfig = spice
            .unmarshal_key_with_validation("database", |config: &DatabaseConfig| {
                config.validate().map_err(|e| ConfigError::invalid_value(e))
            })
            .unwrap();

        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 5432);
    }

    #[test]
    fn test_unmarshal_key_with_validation_failure() {
        use serde::Deserialize;

        #[derive(Deserialize, Debug, PartialEq)]
        struct DatabaseConfig {
            host: String,
            port: u16,
        }

        impl DatabaseConfig {
            fn validate(&self) -> Result<(), String> {
                if self.port < 1024 {
                    return Err("Port should be >= 1024 for non-privileged access".to_string());
                }
                Ok(())
            }
        }

        let mut spice = Spice::new();
        let mut db_config = HashMap::new();
        db_config.insert("host".to_string(), ConfigValue::from("localhost"));
        db_config.insert("port".to_string(), ConfigValue::from(80i64)); // Invalid low port
        spice
            .set("database", ConfigValue::Object(db_config))
            .unwrap();

        let result: Result<DatabaseConfig, _> = spice
            .unmarshal_key_with_validation("database", |config: &DatabaseConfig| {
                config.validate().map_err(|e| ConfigError::invalid_value(e))
            });

        assert!(result.is_err());
        if let Err(ConfigError::InvalidValue(msg)) = result {
            assert_eq!(msg, "Port should be >= 1024 for non-privileged access");
        } else {
            panic!("Expected InvalidValue error");
        }
    }

    #[test]
    fn test_get_string() {
        let mut spice = Spice::new();

        // Test string value
        spice
            .set("string_key", ConfigValue::String("hello".to_string()))
            .unwrap();
        let value = spice.get_string("string_key").unwrap();
        assert_eq!(value, Some("hello".to_string()));

        // Test integer coercion to string
        spice.set("int_key", ConfigValue::Integer(42)).unwrap();
        let value = spice.get_string("int_key").unwrap();
        assert_eq!(value, Some("42".to_string()));

        // Test boolean coercion to string
        spice.set("bool_key", ConfigValue::Boolean(true)).unwrap();
        let value = spice.get_string("bool_key").unwrap();
        assert_eq!(value, Some("true".to_string()));

        // Test non-existent key
        let value = spice.get_string("nonexistent").unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn test_get_int() {
        let mut spice = Spice::new();

        // Test integer value
        spice.set("int_key", ConfigValue::Integer(42)).unwrap();
        let value = spice.get_int("int_key").unwrap();
        assert_eq!(value, Some(42));

        // Test string value (should fail)
        spice
            .set("string_key", ConfigValue::String("hello".to_string()))
            .unwrap();
        let result = spice.get_int("string_key");
        assert!(result.is_err());
        assert!(result.unwrap_err().is_type_conversion());

        // Test non-existent key
        let value = spice.get_int("nonexistent").unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn test_get_i64() {
        let mut spice = Spice::new();
        spice.set("key", ConfigValue::Integer(42)).unwrap();
        let value = spice.get_i64("key").unwrap();
        assert_eq!(value, Some(42));
    }

    #[test]
    fn test_get_i32() {
        let mut spice = Spice::new();

        // Test valid i32 range
        spice.set("valid_key", ConfigValue::Integer(42)).unwrap();
        let value = spice.get_i32("valid_key").unwrap();
        assert_eq!(value, Some(42));

        // Test i32 overflow
        spice
            .set("overflow_key", ConfigValue::Integer(i64::MAX))
            .unwrap();
        let result = spice.get_i32("overflow_key");
        assert!(result.is_err());
        assert!(result.unwrap_err().is_type_conversion());
    }

    #[test]
    fn test_get_float() {
        let mut spice = Spice::new();

        // Test float value
        spice.set("float_key", ConfigValue::Float(3.14)).unwrap();
        let value = spice.get_float("float_key").unwrap();
        assert_eq!(value, Some(3.14));

        // Test integer to float conversion
        spice.set("int_key", ConfigValue::Integer(42)).unwrap();
        let value = spice.get_float("int_key").unwrap();
        assert_eq!(value, Some(42.0));

        // Test string value (should fail)
        spice
            .set("string_key", ConfigValue::String("hello".to_string()))
            .unwrap();
        let result = spice.get_float("string_key");
        assert!(result.is_err());
        assert!(result.unwrap_err().is_type_conversion());
    }

    #[test]
    fn test_get_f64() {
        let mut spice = Spice::new();
        spice.set("key", ConfigValue::Float(3.14)).unwrap();
        let value = spice.get_f64("key").unwrap();
        assert_eq!(value, Some(3.14));
    }

    #[test]
    fn test_get_f32() {
        let mut spice = Spice::new();

        // Test valid f32 range
        spice.set("valid_key", ConfigValue::Float(3.14)).unwrap();
        let value = spice.get_f32("valid_key").unwrap();
        assert!((value.unwrap() - 3.14f32).abs() < f32::EPSILON);

        // Test f32 overflow (f64::MAX should fail)
        spice
            .set("overflow_key", ConfigValue::Float(f64::MAX))
            .unwrap();
        let result = spice.get_f32("overflow_key");
        assert!(result.is_err());
        assert!(result.unwrap_err().is_type_conversion());
    }

    #[test]
    fn test_get_bool() {
        let mut spice = Spice::new();

        // Test boolean value
        spice.set("bool_key", ConfigValue::Boolean(true)).unwrap();
        let value = spice.get_bool("bool_key").unwrap();
        assert_eq!(value, Some(true));

        // Test string coercion to boolean
        spice
            .set("string_true", ConfigValue::String("true".to_string()))
            .unwrap();
        let value = spice.get_bool("string_true").unwrap();
        assert_eq!(value, Some(true));

        spice
            .set("string_false", ConfigValue::String("false".to_string()))
            .unwrap();
        let value = spice.get_bool("string_false").unwrap();
        assert_eq!(value, Some(false));

        // Test integer coercion to boolean
        spice.set("int_zero", ConfigValue::Integer(0)).unwrap();
        let value = spice.get_bool("int_zero").unwrap();
        assert_eq!(value, Some(false));

        spice.set("int_nonzero", ConfigValue::Integer(42)).unwrap();
        let value = spice.get_bool("int_nonzero").unwrap();
        assert_eq!(value, Some(true));

        // Test invalid string (should fail)
        spice
            .set("invalid_string", ConfigValue::String("maybe".to_string()))
            .unwrap();
        let result = spice.get_bool("invalid_string");
        assert!(result.is_err());
        assert!(result.unwrap_err().is_type_conversion());
    }

    #[test]
    fn test_get_array() {
        let mut spice = Spice::new();

        // Test array value
        let array = vec![
            ConfigValue::String("item1".to_string()),
            ConfigValue::Integer(42),
        ];
        spice
            .set("array_key", ConfigValue::Array(array.clone()))
            .unwrap();
        let value = spice.get_array("array_key").unwrap();
        assert_eq!(value, Some(array));

        // Test non-array value (should fail)
        spice
            .set("string_key", ConfigValue::String("hello".to_string()))
            .unwrap();
        let result = spice.get_array("string_key");
        assert!(result.is_err());
        assert!(result.unwrap_err().is_type_conversion());
    }

    #[test]
    fn test_get_object() {
        let mut spice = Spice::new();

        // Test object value
        let mut object = std::collections::HashMap::new();
        object.insert(
            "key1".to_string(),
            ConfigValue::String("value1".to_string()),
        );
        object.insert("key2".to_string(), ConfigValue::Integer(42));
        spice
            .set("object_key", ConfigValue::Object(object.clone()))
            .unwrap();
        let value = spice.get_object("object_key").unwrap();
        assert_eq!(value, Some(object));

        // Test non-object value (should fail)
        spice
            .set("string_key", ConfigValue::String("hello".to_string()))
            .unwrap();
        let result = spice.get_object("string_key");
        assert!(result.is_err());
        assert!(result.unwrap_err().is_type_conversion());
    }

    #[test]
    fn test_is_set() {
        let mut spice = Spice::new();

        // Test non-existent key
        assert!(!spice.is_set("nonexistent"));

        // Test existing key
        spice
            .set("existing_key", ConfigValue::String("value".to_string()))
            .unwrap();
        assert!(spice.is_set("existing_key"));

        // Test null value (should still be considered set)
        spice.set("null_key", ConfigValue::Null).unwrap();
        assert!(spice.is_set("null_key"));
    }

    #[test]
    fn test_all_keys() {
        let mut spice = Spice::new();

        // Initially no keys
        assert_eq!(spice.all_keys().len(), 0);

        // Add some keys
        spice
            .set("key1", ConfigValue::String("value1".to_string()))
            .unwrap();
        spice.set("key2", ConfigValue::Integer(42)).unwrap();

        let keys = spice.all_keys();
        assert!(keys.contains(&"key1".to_string()));
        assert!(keys.contains(&"key2".to_string()));
    }

    #[test]
    fn test_all_settings() {
        let mut spice = Spice::new();

        // Add some configuration values
        spice
            .set("app.name", ConfigValue::String("test_app".to_string()))
            .unwrap();
        spice.set("app.port", ConfigValue::Integer(8080)).unwrap();
        spice.set("debug", ConfigValue::Boolean(true)).unwrap();

        let settings = spice.all_settings().unwrap();
        // Enhanced all_settings expands nested keys, so we have 2 top-level keys: "app" and "debug"
        assert_eq!(settings.len(), 2);

        // Check the nested app structure
        if let Some(ConfigValue::Object(app_obj)) = settings.get("app") {
            assert_eq!(
                app_obj.get("name"),
                Some(&ConfigValue::String("test_app".to_string()))
            );
            assert_eq!(app_obj.get("port"), Some(&ConfigValue::Integer(8080)));
        } else {
            panic!("Expected app to be an object");
        }

        assert_eq!(settings.get("debug"), Some(&ConfigValue::Boolean(true)));
    }

    #[test]
    fn test_write_config_json() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.json");

        let mut spice = Spice::new();
        spice
            .set("app.name", ConfigValue::String("test_app".to_string()))
            .unwrap();
        spice.set("app.port", ConfigValue::Integer(8080)).unwrap();
        spice.set("debug", ConfigValue::Boolean(true)).unwrap();

        // Write configuration to JSON file
        spice.write_config(&config_path).unwrap();

        // Verify file was created and contains expected content
        assert!(config_path.exists());
        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("test_app"));
        assert!(content.contains("8080"));
        assert!(content.contains("true"));

        // Verify we can parse it back
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        // Enhanced serialization expands nested keys
        assert_eq!(parsed["app"]["name"], "test_app");
        assert_eq!(parsed["app"]["port"], 8080);
        assert_eq!(parsed["debug"], true);
    }

    #[test]
    fn test_write_config_yaml() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.yaml");

        let mut spice = Spice::new();
        spice
            .set(
                "database.host",
                ConfigValue::String("localhost".to_string()),
            )
            .unwrap();
        spice
            .set("database.port", ConfigValue::Integer(5432))
            .unwrap();
        spice
            .set("database.ssl", ConfigValue::Boolean(false))
            .unwrap();

        // Write configuration to YAML file
        spice.write_config(&config_path).unwrap();

        // Verify file was created and contains expected content
        assert!(config_path.exists());
        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("localhost"));
        assert!(content.contains("5432"));
        assert!(content.contains("false"));

        // Verify we can parse it back
        let parsed: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();
        assert_eq!(parsed["database"]["host"], "localhost");
        assert_eq!(parsed["database"]["port"], 5432);
        assert_eq!(parsed["database"]["ssl"], false);
    }

    #[test]
    fn test_write_config_toml() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");

        let mut spice = Spice::new();
        spice
            .set("server.host", ConfigValue::String("0.0.0.0".to_string()))
            .unwrap();
        spice
            .set("server.port", ConfigValue::Integer(3000))
            .unwrap();
        spice
            .set("server.timeout", ConfigValue::Float(30.5))
            .unwrap();

        // Write configuration to TOML file
        spice.write_config(&config_path).unwrap();

        // Verify file was created and contains expected content
        assert!(config_path.exists());
        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("0.0.0.0"));
        assert!(content.contains("3000"));
        assert!(content.contains("30.5"));

        // Verify we can parse it back
        let parsed: toml::Value = toml::from_str(&content).unwrap();
        assert_eq!(
            parsed["server"]["host"],
            toml::Value::String("0.0.0.0".to_string())
        );
        assert_eq!(parsed["server"]["port"], toml::Value::Integer(3000));
        assert_eq!(parsed["server"]["timeout"], toml::Value::Float(30.5));
    }

    #[test]
    fn test_write_config_ini() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.ini");

        let mut spice = Spice::new();
        spice
            .set(
                "global_setting",
                ConfigValue::String("global_value".to_string()),
            )
            .unwrap();

        // Create a section with nested values
        let mut section_data = std::collections::HashMap::new();
        section_data.insert(
            "host".to_string(),
            ConfigValue::String("localhost".to_string()),
        );
        section_data.insert("port".to_string(), ConfigValue::Integer(3306));
        section_data.insert("enabled".to_string(), ConfigValue::Boolean(true));
        spice
            .set("database", ConfigValue::Object(section_data))
            .unwrap();

        // Write configuration to INI file
        spice.write_config(&config_path).unwrap();

        // Verify file was created and contains expected content
        assert!(config_path.exists());
        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("global_setting = global_value"));
        assert!(content.contains("[database]"));
        assert!(content.contains("host = localhost"));
        assert!(content.contains("port = 3306"));
        assert!(content.contains("enabled = true"));
    }

    #[test]
    fn test_write_config_as_format_override() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.txt"); // .txt extension

        let mut spice = Spice::new();
        spice
            .set("app.name", ConfigValue::String("test_app".to_string()))
            .unwrap();
        spice
            .set("app.version", ConfigValue::String("1.0.0".to_string()))
            .unwrap();

        // Write as YAML despite .txt extension
        spice.write_config_as(&config_path, "yaml").unwrap();

        // Verify file was created and contains YAML content
        assert!(config_path.exists());
        let content = fs::read_to_string(&config_path).unwrap();

        // Should be valid YAML
        let parsed: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();
        assert_eq!(parsed["app"]["name"], "test_app");
        assert_eq!(parsed["app"]["version"], "1.0.0");
    }

    #[test]
    fn test_safe_write_config_new_file() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("safe_config.json");

        let mut spice = Spice::new();
        spice.set("safe", ConfigValue::Boolean(true)).unwrap();

        // Should succeed for new file
        spice.safe_write_config(&config_path).unwrap();

        // Verify file was created
        assert!(config_path.exists());
        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("true"));
    }

    #[test]
    fn test_safe_write_config_existing_file() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("existing_config.json");

        // Create existing file
        fs::write(&config_path, "existing content").unwrap();

        let mut spice = Spice::new();
        spice.set("safe", ConfigValue::Boolean(true)).unwrap();

        // Should fail for existing file
        let result = spice.safe_write_config(&config_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().is_io_error());

        // Original file should be unchanged
        let content = fs::read_to_string(&config_path).unwrap();
        assert_eq!(content, "existing content");
    }

    #[test]
    fn test_write_config_unsupported_format() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.unknown");

        let mut spice = Spice::new();
        spice
            .set("test", ConfigValue::String("value".to_string()))
            .unwrap();

        // Should fail for unsupported format
        let result = spice.write_config(&config_path);
        assert!(result.is_err());
        // Enhanced error handling now returns Serialization error with context
        assert!(matches!(result.unwrap_err(), ConfigError::Serialization(_)));
    }

    #[test]
    fn test_write_config_as_unsupported_format() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.txt");

        let mut spice = Spice::new();
        spice
            .set("test", ConfigValue::String("value".to_string()))
            .unwrap();

        // Should fail for unsupported format
        let result = spice.write_config_as(&config_path, "unknown");
        assert!(result.is_err());
        // Enhanced error handling now returns Serialization error with context
        assert!(matches!(result.unwrap_err(), ConfigError::Serialization(_)));
    }

    #[test]
    fn test_write_config_complex_nested_structure() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("complex_config.json");

        let mut spice = Spice::new();

        // Create complex nested structure
        let mut database_config = std::collections::HashMap::new();
        database_config.insert(
            "host".to_string(),
            ConfigValue::String("localhost".to_string()),
        );
        database_config.insert("port".to_string(), ConfigValue::Integer(5432));

        let mut credentials = std::collections::HashMap::new();
        credentials.insert(
            "username".to_string(),
            ConfigValue::String("admin".to_string()),
        );
        credentials.insert(
            "password".to_string(),
            ConfigValue::String("secret".to_string()),
        );
        database_config.insert("credentials".to_string(), ConfigValue::Object(credentials));

        spice
            .set("database", ConfigValue::Object(database_config))
            .unwrap();

        // Create array of servers
        let servers = vec![
            ConfigValue::Object({
                let mut server = std::collections::HashMap::new();
                server.insert("name".to_string(), ConfigValue::String("web1".to_string()));
                server.insert("port".to_string(), ConfigValue::Integer(8080));
                server
            }),
            ConfigValue::Object({
                let mut server = std::collections::HashMap::new();
                server.insert("name".to_string(), ConfigValue::String("web2".to_string()));
                server.insert("port".to_string(), ConfigValue::Integer(8081));
                server
            }),
        ];
        spice.set("servers", ConfigValue::Array(servers)).unwrap();

        // Write and verify
        spice.write_config(&config_path).unwrap();

        assert!(config_path.exists());
        let content = fs::read_to_string(&config_path).unwrap();

        // Parse back and verify structure
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["database"]["host"], "localhost");
        assert_eq!(parsed["database"]["credentials"]["username"], "admin");
        assert_eq!(parsed["servers"][0]["name"], "web1");
        assert_eq!(parsed["servers"][1]["port"], 8081);
    }

    #[test]
    fn test_write_config_with_layer_precedence() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("precedence_config.json");

        let mut spice = Spice::new();

        // Add default layer
        spice
            .set_default(
                "shared_key",
                ConfigValue::String("default_value".to_string()),
            )
            .unwrap();
        spice
            .set_default("default_only", ConfigValue::String("default".to_string()))
            .unwrap();

        // Add explicit layer (higher precedence)
        spice
            .set(
                "shared_key",
                ConfigValue::String("explicit_value".to_string()),
            )
            .unwrap();
        spice
            .set("explicit_only", ConfigValue::String("explicit".to_string()))
            .unwrap();

        // Write configuration
        spice.write_config(&config_path).unwrap();

        assert!(config_path.exists());
        let content = fs::read_to_string(&config_path).unwrap();

        // Parse back and verify precedence is respected
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["shared_key"], "explicit_value"); // Explicit should win
        assert_eq!(parsed["default_only"], "default");
        assert_eq!(parsed["explicit_only"], "explicit");
    }

    #[test]
    fn test_write_config_round_trip() {

        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("round_trip.json");

        let mut original_viper = Spice::new();
        original_viper
            .set(
                "app.name",
                ConfigValue::String("round_trip_test".to_string()),
            )
            .unwrap();
        original_viper
            .set("app.port", ConfigValue::Integer(9000))
            .unwrap();
        original_viper
            .set("app.debug", ConfigValue::Boolean(false))
            .unwrap();
        original_viper
            .set("app.timeout", ConfigValue::Float(45.5))
            .unwrap();

        // Write configuration
        original_viper.write_config(&config_path).unwrap();

        // Load configuration into new Spice instance
        let mut loaded_viper = Spice::new();
        loaded_viper.set_config_file(&config_path).unwrap();

        // Verify all values match
        assert_eq!(
            loaded_viper.get_string("app.name").unwrap(),
            Some("round_trip_test".to_string())
        );
        assert_eq!(loaded_viper.get_i64("app.port").unwrap(), Some(9000));
        assert_eq!(loaded_viper.get_bool("app.debug").unwrap(), Some(false));
        assert_eq!(loaded_viper.get_f64("app.timeout").unwrap(), Some(45.5));
    }

    #[test]
    fn test_write_config_empty_configuration() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("empty_config.json");

        let spice = Spice::new(); // No configuration set

        // Should write empty object
        spice.write_config(&config_path).unwrap();

        assert!(config_path.exists());
        let content = fs::read_to_string(&config_path).unwrap();

        // Should be valid JSON representing empty object
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed.is_object());
        assert_eq!(parsed.as_object().unwrap().len(), 0);
    }

    #[test]
    fn test_write_config_permission_error() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let readonly_dir = temp_dir.path().join("readonly");
        fs::create_dir(&readonly_dir).unwrap();

        // Make directory read-only (Unix-specific test)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&readonly_dir).unwrap().permissions();
            perms.set_mode(0o444); // Read-only
            fs::set_permissions(&readonly_dir, perms).unwrap();

            let config_path = readonly_dir.join("config.json");
            let mut spice = Spice::new();
            spice
                .set("test", ConfigValue::String("value".to_string()))
                .unwrap();

            // Should fail with IO error
            let result = spice.write_config(&config_path);
            assert!(result.is_err());
            assert!(result.unwrap_err().is_io_error());

            // Restore permissions for cleanup
            let mut perms = fs::metadata(&readonly_dir).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&readonly_dir, perms).unwrap();
        }
    }

    #[test]
    fn test_all_keys_with_values() {
        let mut spice = Spice::new();

        // Initially no keys
        assert_eq!(spice.all_keys().len(), 0);

        // Add some keys
        spice
            .set("key1", ConfigValue::String("value1".to_string()))
            .unwrap();
        spice.set("key2", ConfigValue::Integer(42)).unwrap();

        let keys = spice.all_keys();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"key1".to_string()));
        assert!(keys.contains(&"key2".to_string()));
    }

    #[test]
    fn test_nested_key_access_simple() {
        let mut spice = Spice::new();

        // Create nested object structure
        let mut database_config = HashMap::new();
        database_config.insert(
            "host".to_string(),
            ConfigValue::String("localhost".to_string()),
        );
        database_config.insert("port".to_string(), ConfigValue::Integer(5432));
        spice
            .set("database", ConfigValue::Object(database_config))
            .unwrap();

        // Test nested access
        let host = spice.get("database.host").unwrap();
        assert_eq!(host, Some(ConfigValue::String("localhost".to_string())));

        let port = spice.get("database.port").unwrap();
        assert_eq!(port, Some(ConfigValue::Integer(5432)));

        // Test non-existent nested key
        let nonexistent = spice.get("database.nonexistent").unwrap();
        assert_eq!(nonexistent, None);
    }

    #[test]
    fn test_nested_key_access_deep() {
        let mut spice = Spice::new();

        // Create deeply nested structure
        let mut server_config = HashMap::new();
        server_config.insert(
            "host".to_string(),
            ConfigValue::String("server1".to_string()),
        );
        server_config.insert("port".to_string(), ConfigValue::Integer(8080));

        let mut database_config = HashMap::new();
        database_config.insert("host".to_string(), ConfigValue::String("db1".to_string()));
        database_config.insert("port".to_string(), ConfigValue::Integer(5432));

        let mut app_config = HashMap::new();
        app_config.insert("server".to_string(), ConfigValue::Object(server_config));
        app_config.insert("database".to_string(), ConfigValue::Object(database_config));

        spice.set("app", ConfigValue::Object(app_config)).unwrap();

        // Test deep nested access
        let server_host = spice.get("app.server.host").unwrap();
        assert_eq!(
            server_host,
            Some(ConfigValue::String("server1".to_string()))
        );

        let db_port = spice.get("app.database.port").unwrap();
        assert_eq!(db_port, Some(ConfigValue::Integer(5432)));
    }

    #[test]
    fn test_array_index_access() {
        let mut spice = Spice::new();

        // Create array structure
        let servers = vec![
            ConfigValue::String("server1.example.com".to_string()),
            ConfigValue::String("server2.example.com".to_string()),
            ConfigValue::String("server3.example.com".to_string()),
        ];
        spice.set("servers", ConfigValue::Array(servers)).unwrap();

        // Test array index access
        let server0 = spice.get("servers.0").unwrap();
        assert_eq!(
            server0,
            Some(ConfigValue::String("server1.example.com".to_string()))
        );

        let server1 = spice.get("servers.1").unwrap();
        assert_eq!(
            server1,
            Some(ConfigValue::String("server2.example.com".to_string()))
        );

        let server2 = spice.get("servers.2").unwrap();
        assert_eq!(
            server2,
            Some(ConfigValue::String("server3.example.com".to_string()))
        );

        // Test out of bounds access
        let server_oob = spice.get("servers.10").unwrap();
        assert_eq!(server_oob, None);
    }

    #[test]
    fn test_mixed_nested_and_array_access() {
        let mut spice = Spice::new();

        // Create mixed structure with objects and arrays
        let mut server1 = HashMap::new();
        server1.insert(
            "host".to_string(),
            ConfigValue::String("server1.example.com".to_string()),
        );
        server1.insert("port".to_string(), ConfigValue::Integer(8080));

        let mut server2 = HashMap::new();
        server2.insert(
            "host".to_string(),
            ConfigValue::String("server2.example.com".to_string()),
        );
        server2.insert("port".to_string(), ConfigValue::Integer(8081));

        let servers = vec![ConfigValue::Object(server1), ConfigValue::Object(server2)];

        let mut config = HashMap::new();
        config.insert("servers".to_string(), ConfigValue::Array(servers));
        spice.set("app", ConfigValue::Object(config)).unwrap();

        // Test mixed access
        let server0_host = spice.get("app.servers.0.host").unwrap();
        assert_eq!(
            server0_host,
            Some(ConfigValue::String("server1.example.com".to_string()))
        );

        let server1_port = spice.get("app.servers.1.port").unwrap();
        assert_eq!(server1_port, Some(ConfigValue::Integer(8081)));

        // Test non-existent path
        let nonexistent = spice.get("app.servers.0.nonexistent").unwrap();
        assert_eq!(nonexistent, None);
    }

    #[test]
    fn test_nested_access_with_exact_key_priority() {
        let mut spice = Spice::new();

        // Set both an exact key and a nested structure
        spice
            .set(
                "database.host",
                ConfigValue::String("exact_key_value".to_string()),
            )
            .unwrap();

        let mut database_config = HashMap::new();
        database_config.insert(
            "host".to_string(),
            ConfigValue::String("nested_value".to_string()),
        );
        spice
            .set("database", ConfigValue::Object(database_config))
            .unwrap();

        // Exact key should take precedence over nested access
        let host = spice.get("database.host").unwrap();
        assert_eq!(
            host,
            Some(ConfigValue::String("exact_key_value".to_string()))
        );
    }

    #[test]
    fn test_sub_configuration() {
        let mut spice = Spice::new();

        // Create nested configuration
        let mut database_config = HashMap::new();
        database_config.insert(
            "host".to_string(),
            ConfigValue::String("localhost".to_string()),
        );
        database_config.insert("port".to_string(), ConfigValue::Integer(5432));
        database_config.insert(
            "username".to_string(),
            ConfigValue::String("admin".to_string()),
        );
        spice
            .set("database", ConfigValue::Object(database_config))
            .unwrap();

        // Create sub-configuration
        let sub_viper = spice.sub("database").unwrap();
        assert!(sub_viper.is_some());
        let mut sub_viper = sub_viper.unwrap();

        // Test direct access in sub-configuration
        let host = sub_viper.get_string("host").unwrap();
        assert_eq!(host, Some("localhost".to_string()));

        let port = sub_viper.get_int("port").unwrap();
        assert_eq!(port, Some(5432));

        let username = sub_viper.get_string("username").unwrap();
        assert_eq!(username, Some("admin".to_string()));

        // Test non-existent key in sub-configuration
        let nonexistent = sub_viper.get("nonexistent").unwrap();
        assert_eq!(nonexistent, None);
    }

    #[test]
    fn test_sub_configuration_non_object() {
        let mut spice = Spice::new();

        // Set a non-object value
        spice
            .set(
                "simple_key",
                ConfigValue::String("simple_value".to_string()),
            )
            .unwrap();

        // Sub-configuration should return None for non-object values
        let sub_viper = spice.sub("simple_key").unwrap();
        assert!(sub_viper.is_none());
    }

    #[test]
    fn test_sub_configuration_nonexistent_key() {
        let spice = Spice::new();

        // Sub-configuration should return None for non-existent keys
        let sub_viper = spice.sub("nonexistent").unwrap();
        assert!(sub_viper.is_none());
    }

    #[test]
    fn test_nested_sub_configuration() {
        let mut spice = Spice::new();

        // Create deeply nested structure
        let mut server_config = HashMap::new();
        server_config.insert(
            "host".to_string(),
            ConfigValue::String("localhost".to_string()),
        );
        server_config.insert("port".to_string(), ConfigValue::Integer(8080));

        let mut app_config = HashMap::new();
        app_config.insert("server".to_string(), ConfigValue::Object(server_config));

        spice.set("app", ConfigValue::Object(app_config)).unwrap();

        // Create sub-configuration for app
        let app_viper = spice.sub("app").unwrap().unwrap();

        // Create nested sub-configuration for server
        let mut server_viper = app_viper.sub("server").unwrap().unwrap();

        // Test access in nested sub-configuration
        let host = server_viper.get_string("host").unwrap();
        assert_eq!(host, Some("localhost".to_string()));

        let port = server_viper.get_int("port").unwrap();
        assert_eq!(port, Some(8080));
    }

    #[test]
    fn test_custom_key_delimiter() {
        let mut spice = Spice::new();
        spice.set_key_delimiter("::");

        // Create nested structure
        let mut database_config = HashMap::new();
        database_config.insert(
            "host".to_string(),
            ConfigValue::String("localhost".to_string()),
        );
        spice
            .set("database", ConfigValue::Object(database_config))
            .unwrap();

        // Test nested access with custom delimiter
        let host = spice.get("database::host").unwrap();
        assert_eq!(host, Some(ConfigValue::String("localhost".to_string())));

        // Test that dot notation doesn't work with custom delimiter
        let host_dot = spice.get("database.host").unwrap();
        assert_eq!(host_dot, None);
    }

    #[test]
    fn test_parse_key() {
        let spice = Spice::new();

        // Test simple key
        let parts = spice.parse_key("simple");
        assert_eq!(parts, vec![KeyPart::Key("simple".to_string())]);

        // Test nested key
        let parts = spice.parse_key("database.host");
        assert_eq!(
            parts,
            vec![
                KeyPart::Key("database".to_string()),
                KeyPart::Key("host".to_string())
            ]
        );

        // Test array index
        let parts = spice.parse_key("servers.0");
        assert_eq!(
            parts,
            vec![KeyPart::Key("servers".to_string()), KeyPart::Index(0)]
        );

        // Test mixed
        let parts = spice.parse_key("app.servers.0.host");
        assert_eq!(
            parts,
            vec![
                KeyPart::Key("app".to_string()),
                KeyPart::Key("servers".to_string()),
                KeyPart::Index(0),
                KeyPart::Key("host".to_string())
            ]
        );
    }

    #[test]
    fn test_traverse_nested_value() {
        let spice = Spice::new();

        // Create test structure
        let mut server = HashMap::new();
        server.insert(
            "host".to_string(),
            ConfigValue::String("localhost".to_string()),
        );
        server.insert("port".to_string(), ConfigValue::Integer(8080));

        let servers = vec![ConfigValue::Object(server)];
        let root = ConfigValue::Array(servers);

        // Test traversal
        let path = vec![KeyPart::Index(0), KeyPart::Key("host".to_string())];
        let result = spice.traverse_nested_value(&root, &path);
        assert_eq!(result, Some(ConfigValue::String("localhost".to_string())));

        // Test invalid path
        let path = vec![KeyPart::Index(1), KeyPart::Key("host".to_string())];
        let result = spice.traverse_nested_value(&root, &path);
        assert_eq!(result, None);

        // Test empty path
        let path = vec![];
        let result = spice.traverse_nested_value(&root, &path);
        assert_eq!(result, Some(root));
    }

    #[test]
    fn test_layer_precedence_in_get_operations() {
        let mut spice = Spice::new();

        // Add layers with different priorities
        let config_layer = Box::new(
            MockConfigLayer::new("config", LayerPriority::ConfigFile)
                .with_value(
                    "shared_key",
                    ConfigValue::String("config_value".to_string()),
                )
                .with_value(
                    "config_only",
                    ConfigValue::String("config_only_value".to_string()),
                ),
        );
        spice.add_layer(config_layer);

        let env_layer = Box::new(
            MockConfigLayer::new("env", LayerPriority::Environment)
                .with_value("shared_key", ConfigValue::String("env_value".to_string()))
                .with_value(
                    "env_only",
                    ConfigValue::String("env_only_value".to_string()),
                ),
        );
        spice.add_layer(env_layer);

        // Explicit set (highest priority)
        spice
            .set(
                "shared_key",
                ConfigValue::String("explicit_value".to_string()),
            )
            .unwrap();

        // Test precedence: explicit > env > config
        assert_eq!(
            spice.get_string("shared_key").unwrap(),
            Some("explicit_value".to_string())
        );
        assert_eq!(
            spice.get_string("env_only").unwrap(),
            Some("env_only_value".to_string())
        );
        assert_eq!(
            spice.get_string("config_only").unwrap(),
            Some("config_only_value".to_string())
        );
    }

    #[test]
    fn test_set_default() {
        let mut spice = Spice::new();

        // Set a default value
        spice
            .set_default("database.host", ConfigValue::from("localhost"))
            .unwrap();
        spice
            .set_default("database.port", ConfigValue::from(5432i64))
            .unwrap();

        // Verify defaults are accessible
        assert_eq!(
            spice.get_string("database.host").unwrap(),
            Some("localhost".to_string())
        );
        assert_eq!(spice.get_i64("database.port").unwrap(), Some(5432));

        // Verify default layer was created with correct priority
        let layer_info = spice.layer_info();
        assert!(layer_info
            .iter()
            .any(|(name, priority)| name == "defaults" && *priority == LayerPriority::Defaults));
    }

    #[test]
    fn test_set_defaults_bulk() {
        let mut spice = Spice::new();

        // Set multiple defaults at once
        let mut defaults = HashMap::new();
        defaults.insert("server.host".to_string(), ConfigValue::from("0.0.0.0"));
        defaults.insert("server.port".to_string(), ConfigValue::from(8080i64));
        defaults.insert("server.ssl".to_string(), ConfigValue::from(false));
        defaults.insert("database.timeout".to_string(), ConfigValue::from(30i64));

        spice.set_defaults(defaults).unwrap();

        // Verify all defaults are accessible
        assert_eq!(
            spice.get_string("server.host").unwrap(),
            Some("0.0.0.0".to_string())
        );
        assert_eq!(spice.get_i64("server.port").unwrap(), Some(8080));
        assert_eq!(spice.get_bool("server.ssl").unwrap(), Some(false));
        assert_eq!(spice.get_i64("database.timeout").unwrap(), Some(30));

        // Verify only one default layer was created
        let layer_info = spice.layer_info();
        let default_layers: Vec<_> = layer_info
            .iter()
            .filter(|(name, _)| name == "defaults")
            .collect();
        assert_eq!(default_layers.len(), 1);
    }

    #[test]
    fn test_default_precedence() {
        let mut spice = Spice::new();

        // Set a default value
        spice
            .set_default("key", ConfigValue::from("default_value"))
            .unwrap();
        assert_eq!(
            spice.get_string("key").unwrap(),
            Some("default_value".to_string())
        );

        // Override with explicit value (higher precedence)
        spice
            .set("key", ConfigValue::from("explicit_value"))
            .unwrap();
        assert_eq!(
            spice.get_string("key").unwrap(),
            Some("explicit_value".to_string())
        );

        // Add a config file layer (higher precedence than defaults, lower than explicit)
        let config_layer = Box::new(
            MockConfigLayer::new("config", LayerPriority::ConfigFile)
                .with_value("key", ConfigValue::from("config_value")),
        );
        spice.add_layer(config_layer);

        // Explicit should still win
        assert_eq!(
            spice.get_string("key").unwrap(),
            Some("explicit_value".to_string())
        );

        // Remove explicit layer and config should win over default
        spice.remove_layers_by_priority(LayerPriority::Explicit);
        assert_eq!(
            spice.get_string("key").unwrap(),
            Some("config_value".to_string())
        );

        // Remove config layer and default should be used
        spice.remove_layers_by_priority(LayerPriority::ConfigFile);
        assert_eq!(
            spice.get_string("key").unwrap(),
            Some("default_value".to_string())
        );
    }

    #[test]
    fn test_multiple_default_operations() {
        let mut spice = Spice::new();

        // Set individual defaults
        spice
            .set_default("key1", ConfigValue::from("value1"))
            .unwrap();
        spice
            .set_default("key2", ConfigValue::from("value2"))
            .unwrap();

        // Set bulk defaults
        let mut bulk_defaults = HashMap::new();
        bulk_defaults.insert("key3".to_string(), ConfigValue::from("value3"));
        bulk_defaults.insert("key4".to_string(), ConfigValue::from("value4"));
        spice.set_defaults(bulk_defaults).unwrap();

        // Override one of the individual defaults
        spice
            .set_default("key1", ConfigValue::from("updated_value1"))
            .unwrap();

        // Verify all values
        assert_eq!(
            spice.get_string("key1").unwrap(),
            Some("updated_value1".to_string())
        );
        assert_eq!(
            spice.get_string("key2").unwrap(),
            Some("value2".to_string())
        );
        assert_eq!(
            spice.get_string("key3").unwrap(),
            Some("value3".to_string())
        );
        assert_eq!(
            spice.get_string("key4").unwrap(),
            Some("value4".to_string())
        );

        // Verify still only one default layer
        let layer_info = spice.layer_info();
        let default_layers: Vec<_> = layer_info
            .iter()
            .filter(|(name, _)| name == "defaults")
            .collect();
        assert_eq!(default_layers.len(), 1);
    }

    #[test]
    fn test_defaults_with_nested_keys() {
        let mut spice = Spice::new();

        // Set nested default values
        spice
            .set_default("database.connection.host", ConfigValue::from("localhost"))
            .unwrap();
        spice
            .set_default("database.connection.port", ConfigValue::from(5432i64))
            .unwrap();
        spice
            .set_default("database.pool.max_size", ConfigValue::from(10i64))
            .unwrap();

        // Verify nested access works with defaults
        assert_eq!(
            spice.get_string("database.connection.host").unwrap(),
            Some("localhost".to_string())
        );
        assert_eq!(
            spice.get_i64("database.connection.port").unwrap(),
            Some(5432)
        );
        assert_eq!(spice.get_i64("database.pool.max_size").unwrap(), Some(10));

        // Test that defaults work with sub-configurations
        // Note: This will only work if we have a nested object structure, not just dot-notation keys
        // For now, just verify the keys exist
        assert!(spice.is_set("database.connection.host"));
        assert!(spice.is_set("database.connection.port"));
        assert!(spice.is_set("database.pool.max_size"));
    }

    #[test]
    fn test_defaults_with_different_value_types() {
        let mut spice = Spice::new();

        // Set defaults with various types
        spice
            .set_default("string_val", ConfigValue::from("hello"))
            .unwrap();
        spice
            .set_default("int_val", ConfigValue::from(42i64))
            .unwrap();
        spice
            .set_default("float_val", ConfigValue::from(3.14))
            .unwrap();
        spice
            .set_default("bool_val", ConfigValue::from(true))
            .unwrap();
        spice.set_default("null_val", ConfigValue::Null).unwrap();

        // Create array and object defaults
        let array_val =
            ConfigValue::Array(vec![ConfigValue::from("item1"), ConfigValue::from("item2")]);
        spice.set_default("array_val", array_val).unwrap();

        let mut obj = HashMap::new();
        obj.insert("nested_key".to_string(), ConfigValue::from("nested_value"));
        spice
            .set_default("object_val", ConfigValue::Object(obj))
            .unwrap();

        // Verify all types work correctly
        assert_eq!(
            spice.get_string("string_val").unwrap(),
            Some("hello".to_string())
        );
        assert_eq!(spice.get_i64("int_val").unwrap(), Some(42));
        assert_eq!(spice.get_f64("float_val").unwrap(), Some(3.14));
        assert_eq!(spice.get_bool("bool_val").unwrap(), Some(true));
        assert_eq!(spice.get("null_val").unwrap(), Some(ConfigValue::Null));

        let array = spice.get_array("array_val").unwrap().unwrap();
        assert_eq!(array.len(), 2);
        assert_eq!(array[0], ConfigValue::from("item1"));

        let obj = spice.get_object("object_val").unwrap().unwrap();
        assert_eq!(
            obj.get("nested_key"),
            Some(&ConfigValue::from("nested_value"))
        );
    }

    // File discovery tests
    #[test]
    fn test_find_config_file_empty_name() {
        let spice = Spice::new();
        let result = spice.find_config_file().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_find_config_file_no_paths() {
        let mut spice = Spice::new();
        spice.set_config_name("nonexistent");

        let result = spice.find_config_file().unwrap();
        // Should return None since no config file exists
        assert!(result.is_none());
    }

    #[test]
    fn test_find_config_file_with_temp_file() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"{"test_key": "test_value"}"#;
        let config_file = temp_dir.path().join("test_config.json");
        fs::write(&config_file, config_content).unwrap();

        let mut spice = Spice::new();
        spice.set_config_name("test_config");
        spice.add_config_path(temp_dir.path());

        let result = spice.find_config_file().unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), config_file);
    }

    #[test]
    fn test_find_config_file_multiple_extensions() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();

        // Create multiple config files with different extensions
        let json_content = r#"{"format": "json"}"#;
        let yaml_content = "format: yaml";
        let toml_content = "format = \"toml\"";

        fs::write(temp_dir.path().join("app.json"), json_content).unwrap();
        fs::write(temp_dir.path().join("app.yaml"), yaml_content).unwrap();
        fs::write(temp_dir.path().join("app.toml"), toml_content).unwrap();

        let mut spice = Spice::new();
        spice.set_config_name("app");
        spice.add_config_path(temp_dir.path());

        let result = spice.find_config_file().unwrap();
        assert!(result.is_some());

        // Should find the first one (json comes first in the extension list)
        let found_file = result.unwrap();
        assert_eq!(found_file.extension().unwrap(), "json");
    }

    #[test]
    fn test_find_config_file_priority_order() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();

        // Create config files in both directories
        let config_content1 = r#"{"source": "dir1"}"#;
        let config_content2 = r#"{"source": "dir2"}"#;

        fs::write(temp_dir1.path().join("priority_test.json"), config_content1).unwrap();
        fs::write(temp_dir2.path().join("priority_test.json"), config_content2).unwrap();

        let mut spice = Spice::new();
        spice.set_config_name("priority_test");
        spice.add_config_path(temp_dir1.path()); // Added first, should have priority
        spice.add_config_path(temp_dir2.path());

        let result = spice.find_config_file().unwrap();
        assert!(result.is_some());

        // Should find the file from the first directory
        let found_file = result.unwrap();
        assert!(found_file.starts_with(temp_dir1.path()));
    }

    #[test]
    fn test_find_all_config_files() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();

        // Create config files in both directories with different extensions
        fs::write(temp_dir1.path().join("multi.json"), r#"{"source": "dir1"}"#).unwrap();
        fs::write(temp_dir1.path().join("multi.yaml"), "source: dir1_yaml").unwrap();
        fs::write(temp_dir2.path().join("multi.toml"), "source = \"dir2\"").unwrap();

        let mut spice = Spice::new();
        spice.set_config_name("multi");
        spice.add_config_path(temp_dir1.path());
        spice.add_config_path(temp_dir2.path());

        let result = spice.find_all_config_files().unwrap();
        assert_eq!(result.len(), 3); // Should find all three files

        // Verify all files are found
        let file_names: Vec<String> = result
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        assert!(file_names.contains(&"multi.json".to_string()));
        assert!(file_names.contains(&"multi.yaml".to_string()));
        assert!(file_names.contains(&"multi.toml".to_string()));
    }

    #[test]
    fn test_read_in_config_success() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"{"database": {"host": "localhost", "port": 5432}}"#;
        let config_file = temp_dir.path().join("read_test.json");
        fs::write(&config_file, config_content).unwrap();

        let mut spice = Spice::new();
        spice.set_config_name("read_test");
        spice.add_config_path(temp_dir.path());

        let result = spice.read_in_config();
        assert!(result.is_ok());

        // Verify the configuration was loaded
        assert_eq!(
            spice.get_string("database.host").unwrap(),
            Some("localhost".to_string())
        );
        assert_eq!(spice.get_i64("database.port").unwrap(), Some(5432));
    }

    #[test]
    fn test_read_in_config_file_not_found() {
        let mut spice = Spice::new();
        spice.set_config_name("nonexistent");
        spice.add_config_path("/nonexistent/path");

        let result = spice.read_in_config();
        assert!(result.is_err());

        if let Err(ConfigError::KeyNotFound { key }) = result {
            assert!(key.contains("nonexistent"));
        } else {
            panic!("Expected KeyNotFound error");
        }
    }

    #[test]
    fn test_set_config_file_direct() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"{"direct": "load", "value": 42}"#;
        let config_file = temp_dir.path().join("direct.json");
        fs::write(&config_file, config_content).unwrap();

        let mut spice = Spice::new();
        let result = spice.set_config_file(&config_file);
        assert!(result.is_ok());

        // Verify the configuration was loaded
        assert_eq!(
            spice.get_string("direct").unwrap(),
            Some("load".to_string())
        );
        assert_eq!(spice.get_i64("value").unwrap(), Some(42));
    }

    #[test]
    fn test_merge_in_config() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();

        // Create multiple config files with overlapping keys
        let config1 = r#"{"shared": "from_json", "json_only": "json_value"}"#;
        let config2 = "shared: from_yaml\nyaml_only: yaml_value";
        let config3 = "shared = \"from_toml\"\ntoml_only = \"toml_value\"";

        fs::write(temp_dir.path().join("merge.json"), config1).unwrap();
        fs::write(temp_dir.path().join("merge.yaml"), config2).unwrap();
        fs::write(temp_dir.path().join("merge.toml"), config3).unwrap();

        let mut spice = Spice::new();
        spice.set_config_name("merge");
        spice.add_config_path(temp_dir.path());

        let merged_count = spice.merge_in_config().unwrap();
        assert_eq!(merged_count, 3);

        // Verify all unique keys are present
        assert!(spice.is_set("json_only"));
        assert!(spice.is_set("yaml_only"));
        assert!(spice.is_set("toml_only"));

        // The shared key should have the value from the first file found (JSON)
        assert_eq!(
            spice.get_string("shared").unwrap(),
            Some("from_json".to_string())
        );
    }

    #[test]
    fn test_load_config_file_invalid_format() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let invalid_json = r#"{"invalid": json content}"#; // Missing quotes around "json"
        let config_file = temp_dir.path().join("invalid.json");
        fs::write(&config_file, invalid_json).unwrap();

        let mut spice = Spice::new();
        let result = spice.load_config_file(&config_file);
        assert!(result.is_err());

        // Should be a parse error
        match result {
            Err(ConfigError::Parse {
                source_name,
                message: _,
            }) => {
                // The source_name might be the file path, not just "JSON"
                assert!(source_name.contains("JSON") || source_name.contains("invalid.json"));
            }
            Err(e) => panic!("Expected Parse error, got: {:?}", e),
            Ok(_) => panic!("Expected error for invalid JSON, but got success"),
        }
    }

    #[test]
    fn test_get_standard_config_paths() {
        let spice = Spice::new();
        let paths = spice.get_standard_config_paths().unwrap();

        // Should always include current directory
        assert!(paths.contains(&PathBuf::from(".")));

        // Should include some system paths (exact paths depend on OS)
        assert!(paths.len() > 1);
    }

    #[test]
    fn test_config_file_precedence_with_explicit_set() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"{"precedence_test": "from_file"}"#;
        let config_file = temp_dir.path().join("precedence.json");
        fs::write(&config_file, config_content).unwrap();

        let mut spice = Spice::new();

        // Load config file first
        spice.load_config_file(&config_file).unwrap();
        assert_eq!(
            spice.get_string("precedence_test").unwrap(),
            Some("from_file".to_string())
        );

        // Set explicit value (should override file)
        spice
            .set("precedence_test", ConfigValue::from("explicit_value"))
            .unwrap();
        assert_eq!(
            spice.get_string("precedence_test").unwrap(),
            Some("explicit_value".to_string())
        );
    }

    #[test]
    fn test_multiple_format_support() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();

        // Test each supported format
        let formats = vec![
            ("test.json", r#"{"format": "json", "number": 42}"#),
            ("test.yaml", "format: yaml\nnumber: 42"),
            ("test.toml", "format = \"toml\"\nnumber = 42"),
            ("test.ini", "[section]\nformat = ini\nnumber = 42"),
        ];

        for (filename, content) in formats {
            let config_file = temp_dir.path().join(filename);
            fs::write(&config_file, content).unwrap();

            let mut spice = Spice::new();
            let result = spice.load_config_file(&config_file);
            assert!(result.is_ok(), "Failed to load {}: {:?}", filename, result);

            // Verify content was parsed correctly
            if filename.ends_with(".ini") {
                // INI files have sections
                assert_eq!(
                    spice.get_string("section.format").unwrap(),
                    Some("ini".to_string())
                );
                assert_eq!(spice.get_i64("section.number").unwrap(), Some(42));
            } else {
                assert!(spice.is_set("format"));
                assert_eq!(spice.get_i64("number").unwrap(), Some(42));
            }
        }
    }

    #[test]
    fn test_file_watching_integration() {
        use std::fs;
        use std::sync::{Arc, Mutex};
        use std::thread;
        use std::time::Duration;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        // Create initial config file
        fs::write(&config_path, r#"{"key": "initial_value"}"#).unwrap();

        let mut spice = Spice::new();
        spice.set_config_file(&config_path).unwrap();

        // Verify initial value
        assert_eq!(
            spice.get_string("key").unwrap(),
            Some("initial_value".to_string())
        );

        // Enable file watching
        spice.watch_config().unwrap();
        assert!(spice.is_watching());

        // Register callback to track changes
        let change_count = Arc::new(Mutex::new(0));
        let change_count_clone = Arc::clone(&change_count);

        spice
            .on_config_change(move || {
                let mut count = change_count_clone.lock().unwrap();
                *count += 1;
            })
            .unwrap();

        // Modify the file
        fs::write(&config_path, r#"{"key": "updated_value"}"#).unwrap();

        // Give some time for the file watcher to detect the change
        thread::sleep(Duration::from_millis(100));

        // Access configuration to trigger reload and callback
        assert_eq!(
            spice.get_string("key").unwrap(),
            Some("updated_value".to_string())
        );

        // Check that callback was called
        let final_count = *change_count.lock().unwrap();
        assert!(
            final_count > 0,
            "Configuration change callback should have been called"
        );

        // Stop watching
        spice.stop_watching();
        assert!(!spice.is_watching());
    }

    #[test]
    fn test_on_config_change_without_watching() {
        let mut spice = Spice::new();

        // Try to register callback without enabling file watching
        let result = spice.on_config_change(|| {});
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("File watching is not enabled"));
    }

    #[test]
    fn test_multiple_config_change_callbacks() {
        use std::fs;
        use std::sync::{Arc, Mutex};
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        fs::write(&config_path, "{}").unwrap();

        let mut spice = Spice::new();
        spice.set_config_file(&config_path).unwrap();
        spice.watch_config().unwrap();

        let callback1_called = Arc::new(Mutex::new(false));
        let callback2_called = Arc::new(Mutex::new(false));

        let callback1_called_clone = Arc::clone(&callback1_called);
        let callback2_called_clone = Arc::clone(&callback2_called);

        // Register multiple callbacks
        spice
            .on_config_change(move || {
                *callback1_called_clone.lock().unwrap() = true;
            })
            .unwrap();

        spice
            .on_config_change(move || {
                *callback2_called_clone.lock().unwrap() = true;
            })
            .unwrap();

        // Write some configuration to trigger callbacks
        fs::write(&config_path, r#"{"test": "value"}"#).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Access configuration to trigger reload and callbacks
        let _ = spice.get_string("test").unwrap();

        // Both callbacks should have been called
        assert!(*callback1_called.lock().unwrap());
        assert!(*callback2_called.lock().unwrap());

        spice.stop_watching();
    }

    #[test]
    fn test_watched_config_files() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        fs::write(&config_path, "{}").unwrap();

        let mut spice = Spice::new();
        assert_eq!(spice.watched_config_files().len(), 0);

        spice.set_config_file(&config_path).unwrap();
        spice.watch_config().unwrap();

        let watched_files = spice.watched_config_files();
        assert_eq!(watched_files.len(), 1);
        assert_eq!(watched_files[0], config_path);

        spice.stop_watching();
        assert_eq!(spice.watched_config_files().len(), 0);
    }

    #[test]
    fn test_serialization_with_special_float_values() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("special_floats.json");

        let mut spice = Spice::new();

        // Add special float values that need optimization
        spice
            .set("normal_float", ConfigValue::Float(3.14159))
            .unwrap();
        spice.set("zero_float", ConfigValue::Float(0.0)).unwrap();
        spice
            .set("negative_zero", ConfigValue::Float(-0.0))
            .unwrap();
        spice
            .set("nan_float", ConfigValue::Float(f64::NAN))
            .unwrap();
        spice
            .set("infinity_float", ConfigValue::Float(f64::INFINITY))
            .unwrap();
        spice
            .set("neg_infinity_float", ConfigValue::Float(f64::NEG_INFINITY))
            .unwrap();

        // Write configuration - should handle special values
        spice.write_config(&config_path).unwrap();

        assert!(config_path.exists());
        let content = fs::read_to_string(&config_path).unwrap();

        // Parse back and verify special values were handled
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["normal_float"], 3.14159);
        assert_eq!(parsed["zero_float"], 0.0);

        // NaN and infinity should be converted to strings
        assert_eq!(parsed["nan_float"], "NaN");
        assert_eq!(parsed["infinity_float"], "inf");
        assert_eq!(parsed["neg_infinity_float"], "-inf");
    }

    #[test]
    fn test_serialization_configuration_merging() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("merged_config.json");

        let mut spice = Spice::new();

        // Add values from different layers to test merging
        spice
            .set_default("app.name", ConfigValue::String("default-app".to_string()))
            .unwrap();
        spice
            .set_default("app.version", ConfigValue::String("1.0.0".to_string()))
            .unwrap();
        spice
            .set_default("app.debug", ConfigValue::Boolean(false))
            .unwrap();

        // Override some defaults with explicit values
        spice
            .set("app.name", ConfigValue::String("my-app".to_string()))
            .unwrap();
        spice.set("app.debug", ConfigValue::Boolean(true)).unwrap();

        // Add additional explicit values
        spice
            .set(
                "database.host",
                ConfigValue::String("localhost".to_string()),
            )
            .unwrap();
        spice
            .set("database.port", ConfigValue::Integer(5432))
            .unwrap();

        // Write configuration - should merge all layers properly
        spice.write_config(&config_path).unwrap();

        assert!(config_path.exists());
        let content = fs::read_to_string(&config_path).unwrap();

        // Parse back and verify merging worked correctly
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Explicit values should override defaults
        assert_eq!(parsed["app"]["name"], "my-app");
        assert_eq!(parsed["app"]["debug"], true);

        // Default values should be preserved when not overridden
        assert_eq!(parsed["app"]["version"], "1.0.0");

        // Explicit-only values should be present
        assert_eq!(parsed["database"]["host"], "localhost");
        assert_eq!(parsed["database"]["port"], 5432);
    }

    #[test]
    fn test_write_config_as_with_enhanced_error_handling() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("subdir").join("config.yaml");

        let mut spice = Spice::new();
        spice
            .set("test.key", ConfigValue::String("test_value".to_string()))
            .unwrap();

        // Should create parent directories automatically
        spice.write_config_as(&config_path, "yaml").unwrap();

        assert!(config_path.exists());
        assert!(config_path.parent().unwrap().exists());

        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("test_value"));
    }

    #[test]
    fn test_write_config_as_unsupported_format_enhanced_error() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.txt");

        let mut spice = Spice::new();
        spice
            .set("test", ConfigValue::String("value".to_string()))
            .unwrap();

        // Should fail with enhanced error message
        let result = spice.write_config_as(&config_path, "unsupported");
        assert!(result.is_err());

        if let Err(crate::error::ConfigError::Serialization(msg)) = result {
            assert!(msg.contains("Failed to detect parser for format 'unsupported'"));
        } else {
            panic!("Expected Serialization error with enhanced message");
        }
    }

    #[test]
    fn test_serialization_nested_key_expansion() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("nested_expansion.json");

        let mut spice = Spice::new();

        // Set nested keys using dot notation
        spice
            .set(
                "app.database.host",
                ConfigValue::String("localhost".to_string()),
            )
            .unwrap();
        spice
            .set("app.database.port", ConfigValue::Integer(5432))
            .unwrap();
        spice
            .set(
                "app.server.host",
                ConfigValue::String("0.0.0.0".to_string()),
            )
            .unwrap();
        spice
            .set("app.server.port", ConfigValue::Integer(8080))
            .unwrap();

        // Write configuration - should expand nested keys properly
        spice.write_config(&config_path).unwrap();

        assert!(config_path.exists());
        let content = fs::read_to_string(&config_path).unwrap();

        // Parse back and verify nested structure
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["app"]["database"]["host"], "localhost");
        assert_eq!(parsed["app"]["database"]["port"], 5432);
        assert_eq!(parsed["app"]["server"]["host"], "0.0.0.0");
        assert_eq!(parsed["app"]["server"]["port"], 8080);
    }

    #[test]
    fn test_serialization_format_specific_handling() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();

        let mut spice = Spice::new();
        spice
            .set("string_key", ConfigValue::String("hello world".to_string()))
            .unwrap();
        spice.set("integer_key", ConfigValue::Integer(42)).unwrap();
        spice.set("float_key", ConfigValue::Float(3.14159)).unwrap();
        spice
            .set("boolean_key", ConfigValue::Boolean(true))
            .unwrap();
        spice.set("null_key", ConfigValue::Null).unwrap();

        // Test JSON serialization
        let json_path = temp_dir.path().join("test.json");
        spice.write_config_as(&json_path, "json").unwrap();
        let json_content = fs::read_to_string(&json_path).unwrap();
        assert!(json_content.contains("\"hello world\""));
        assert!(json_content.contains("42"));
        assert!(json_content.contains("3.14159"));
        assert!(json_content.contains("true"));
        assert!(json_content.contains("null"));

        // Test YAML serialization
        let yaml_path = temp_dir.path().join("test.yaml");
        spice.write_config_as(&yaml_path, "yaml").unwrap();
        let yaml_content = fs::read_to_string(&yaml_path).unwrap();
        assert!(yaml_content.contains("hello world"));
        assert!(yaml_content.contains("42"));
        assert!(yaml_content.contains("3.14159"));
        assert!(yaml_content.contains("true"));

        // Test TOML serialization
        let toml_path = temp_dir.path().join("test.toml");
        spice.write_config_as(&toml_path, "toml").unwrap();
        let toml_content = fs::read_to_string(&toml_path).unwrap();
        assert!(toml_content.contains("\"hello world\""));
        assert!(toml_content.contains("42"));
        assert!(toml_content.contains("3.14159"));
        assert!(toml_content.contains("true"));
    }

    #[test]
    fn test_write_config_file_permission_error_enhanced() {



        // Only run on Unix systems where we can control permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let temp_dir = TempDir::new().unwrap();
            let readonly_dir = temp_dir.path().join("readonly");
            fs::create_dir(&readonly_dir).unwrap();

            // Make directory read-only
            let mut perms = fs::metadata(&readonly_dir).unwrap().permissions();
            perms.set_mode(0o444);
            fs::set_permissions(&readonly_dir, perms).unwrap();

            let config_path = readonly_dir.join("config.json");

            let mut spice = Spice::new();
            spice
                .set("test", ConfigValue::String("value".to_string()))
                .unwrap();

            // Should fail with enhanced IO error message
            let result = spice.write_config(&config_path);
            assert!(result.is_err());

            if let Err(crate::error::ConfigError::Io(io_err)) = result {
                let error_msg = io_err.to_string();
                assert!(error_msg.contains("Failed to write configuration to"));
                assert!(error_msg.contains("config.json"));
            } else {
                panic!("Expected IO error with enhanced message");
            }

            // Restore permissions for cleanup
            let mut perms = fs::metadata(&readonly_dir).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&readonly_dir, perms).unwrap();
        }
    }

    #[test]
    fn test_serialization_optimization_recursive() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("recursive_optimization.json");

        let mut spice = Spice::new();

        // Create deeply nested structure with special values
        let mut level1 = std::collections::HashMap::new();
        let mut level2 = std::collections::HashMap::new();
        let mut level3 = std::collections::HashMap::new();

        level3.insert("normal".to_string(), ConfigValue::Float(1.23));
        level3.insert("nan".to_string(), ConfigValue::Float(f64::NAN));
        level3.insert("infinity".to_string(), ConfigValue::Float(f64::INFINITY));

        level2.insert("nested".to_string(), ConfigValue::Object(level3));
        level2.insert(
            "array".to_string(),
            ConfigValue::Array(vec![
                ConfigValue::Float(f64::NAN),
                ConfigValue::Float(f64::INFINITY),
                ConfigValue::Float(2.71),
            ]),
        );

        level1.insert("deep".to_string(), ConfigValue::Object(level2));
        spice.set("root", ConfigValue::Object(level1)).unwrap();

        // Write configuration - should recursively optimize all values
        spice.write_config(&config_path).unwrap();

        assert!(config_path.exists());
        let content = fs::read_to_string(&config_path).unwrap();

        // Parse back and verify recursive optimization
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["root"]["deep"]["nested"]["normal"], 1.23);
        assert_eq!(parsed["root"]["deep"]["nested"]["nan"], "NaN");
        assert_eq!(parsed["root"]["deep"]["nested"]["infinity"], "inf");
        assert_eq!(parsed["root"]["deep"]["array"][0], "NaN");
        assert_eq!(parsed["root"]["deep"]["array"][1], "inf");
        assert_eq!(parsed["root"]["deep"]["array"][2], 2.71);
    }

    #[cfg(feature = "cli")]
    mod flag_binding_tests {
        use super::*;
        use clap::{Arg, Command};
        use std::collections::HashMap;

        fn create_test_cli_app() -> Command {
            Command::new("test")
                .disable_help_flag(true)
                .arg(
                    Arg::new("host")
                        .long("host")
                        .short('h')
                        .value_name("HOST")
                        .action(clap::ArgAction::Set)
                        .help("Database host"),
                )
                .arg(
                    Arg::new("port")
                        .long("port")
                        .short('p')
                        .value_name("PORT")
                        .action(clap::ArgAction::Set)
                        .help("Database port"),
                )
                .arg(
                    Arg::new("verbose")
                        .long("verbose")
                        .short('v')
                        .action(clap::ArgAction::SetTrue)
                        .help("Enable verbose output"),
                )
        }

        #[test]
        fn test_bind_flags_basic() {
            let app = create_test_cli_app();
            let args = vec!["test", "--host", "localhost", "--port", "5432", "--verbose"];
            let matches = app.try_get_matches_from(args).unwrap();

            let mut spice = Spice::new();
            spice.bind_flags(matches);

            // Test string flag
            assert_eq!(
                spice.get_string("host").unwrap(),
                Some("localhost".to_string())
            );

            // Test integer flag (parsed from string)
            assert_eq!(spice.get_i64("port").unwrap(), Some(5432));

            // Test boolean flag
            assert_eq!(spice.get_bool("verbose").unwrap(), Some(true));
        }

        #[test]
        fn test_bind_flags_with_mappings() {
            let app = create_test_cli_app();
            let args = vec!["test", "--host", "localhost", "--port", "5432"];
            let matches = app.try_get_matches_from(args).unwrap();

            let mut mappings = HashMap::new();
            mappings.insert("host".to_string(), "database.host".to_string());
            mappings.insert("port".to_string(), "database.port".to_string());

            let mut spice = Spice::new();
            spice.bind_flags_with_mappings(matches, mappings);

            // Test mapped keys
            assert_eq!(
                spice.get_string("database.host").unwrap(),
                Some("localhost".to_string())
            );
            assert_eq!(spice.get_i64("database.port").unwrap(), Some(5432));

            // Original keys should not be available
            assert_eq!(spice.get_string("host").unwrap(), None);
            assert_eq!(spice.get_i64("port").unwrap(), None);
        }

        #[test]
        fn test_bind_flag_individual() {
            let app = create_test_cli_app();
            let args = vec!["test", "--verbose", "--host", "localhost"];
            let matches = app.try_get_matches_from(args).unwrap();

            let mut spice = Spice::new();
            spice.bind_flags(matches);

            // Initially available under original key
            assert_eq!(spice.get_bool("verbose").unwrap(), Some(true));

            // Bind individual flag to custom key
            spice.bind_flag("verbose", "logging.verbose").unwrap();

            // After binding, should be available under the new key
            assert_eq!(spice.get_bool("logging.verbose").unwrap(), Some(true));
            // Original key should no longer be available since mapping replaces it
            assert_eq!(spice.get_bool("verbose").unwrap(), None);
        }

        #[test]
        fn test_bind_flag_without_flag_layer() {
            let mut spice = Spice::new();

            // Should fail when no flag layer exists
            let result = spice.bind_flag("verbose", "logging.verbose");
            assert!(result.is_err());

            if let Err(ConfigError::UnsupportedOperation(msg)) = result {
                assert!(msg.contains("No flag configuration layer found"));
            } else {
                panic!("Expected UnsupportedOperation error");
            }
        }

        #[test]
        fn test_flag_precedence_over_other_sources() {
            let mut spice = Spice::new();

            // Set default value
            spice
                .set_default("host", ConfigValue::String("default-host".to_string()))
                .unwrap();

            // Add flag layer
            let app = create_test_cli_app();
            let args = vec!["test", "--host", "flag-host"];
            let matches = app.try_get_matches_from(args).unwrap();
            spice.bind_flags(matches);

            // Flag should override default
            assert_eq!(
                spice.get_string("host").unwrap(),
                Some("flag-host".to_string())
            );
        }

        #[test]
        fn test_explicit_set_overrides_flags() {
            let mut spice = Spice::new();

            // Add flag layer
            let app = create_test_cli_app();
            let args = vec!["test", "--host", "flag-host"];
            let matches = app.try_get_matches_from(args).unwrap();
            spice.bind_flags(matches);

            // Explicit set should override flag
            spice
                .set("host", ConfigValue::String("explicit-host".to_string()))
                .unwrap();

            assert_eq!(
                spice.get_string("host").unwrap(),
                Some("explicit-host".to_string())
            );
        }

        #[test]
        fn test_count_flags() {
            let app = Command::new("test").disable_help_flag(true).arg(
                Arg::new("debug")
                    .long("debug")
                    .short('d')
                    .action(clap::ArgAction::Count)
                    .help("Debug level"),
            );

            let args = vec!["test", "-ddd"];
            let matches = app.try_get_matches_from(args).unwrap();

            let mut spice = Spice::new();
            spice.bind_flags(matches);

            // Count flags should be converted to integers
            assert_eq!(spice.get_i64("debug").unwrap(), Some(3));
        }

        #[test]
        fn test_flag_layer_priority() {
            let mut spice = Spice::new();

            // Add layers in different order to test priority sorting
            spice
                .set_default("key", ConfigValue::String("default".to_string()))
                .unwrap();

            let app = create_test_cli_app();
            let args = vec!["test", "--host", "flag-value"];
            let matches = app.try_get_matches_from(args).unwrap();
            spice.bind_flags(matches);

            // Check that layers are properly sorted by priority
            let layer_info = spice.layer_info();
            let flag_layer_index = layer_info
                .iter()
                .position(|(_, priority)| *priority == LayerPriority::Flags);
            let default_layer_index = layer_info
                .iter()
                .position(|(_, priority)| *priority == LayerPriority::Defaults);

            assert!(flag_layer_index.is_some());
            assert!(default_layer_index.is_some());
            assert!(flag_layer_index.unwrap() < default_layer_index.unwrap());
        }

        #[test]
        fn test_multiple_flag_layers() {
            let mut spice = Spice::new();

            // Add first flag layer
            let app1 = Command::new("test1")
                .disable_help_flag(true)
                .arg(Arg::new("host").long("host").action(clap::ArgAction::Set));
            let args1 = vec!["test1", "--host", "host1"];
            let matches1 = app1.try_get_matches_from(args1).unwrap();
            spice.bind_flags(matches1);

            // Add second flag layer
            let app2 = Command::new("test2")
                .disable_help_flag(true)
                .arg(Arg::new("port").long("port").action(clap::ArgAction::Set));
            let args2 = vec!["test2", "--port", "8080"];
            let matches2 = app2.try_get_matches_from(args2).unwrap();
            spice.bind_flags(matches2);

            // Both flags should be available
            assert_eq!(spice.get_string("host").unwrap(), Some("host1".to_string()));
            assert_eq!(spice.get_i64("port").unwrap(), Some(8080));
        }
    }
}
