//! Real-world example: Microservice configuration
//!
//! This example demonstrates configuration management for a microservice
//! with service discovery, health checks, and distributed tracing.

use serde::{Deserialize, Serialize};
use spicex::{ConfigValue, EnvConfigLayer, Spice};
use std::collections::HashMap;
use std::env;

#[derive(Debug, Deserialize, Serialize)]
struct MicroserviceConfig {
    service: ServiceConfig,
    server: ServerConfig,
    database: DatabaseConfig,
    cache: CacheConfig,
    messaging: MessagingConfig,
    observability: ObservabilityConfig,
    security: SecurityConfig,
    #[serde(default)]
    feature_flags: HashMap<String, bool>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ServiceConfig {
    name: String,
    version: String,
    #[serde(default = "default_environment")]
    environment: String,
    #[serde(default = "default_instance_id")]
    instance_id: String,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ServerConfig {
    #[serde(default = "default_host")]
    host: String,
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default = "default_shutdown_timeout")]
    shutdown_timeout: u64,
    health_check: HealthCheckConfig,
}

#[derive(Debug, Deserialize, Serialize)]
struct HealthCheckConfig {
    #[serde(default = "default_health_path")]
    path: String,
    #[serde(default = "default_health_interval")]
    interval: u64,
    #[serde(default = "default_health_timeout")]
    timeout: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct DatabaseConfig {
    host: String,
    port: u16,
    database: String,
    username: String,
    password: String,
    #[serde(default = "default_pool_size")]
    pool_size: u32,
    #[serde(default = "default_max_idle")]
    max_idle: u32,
    #[serde(default = "default_connection_timeout")]
    connection_timeout: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct CacheConfig {
    #[serde(default = "default_cache_type")]
    cache_type: String, // "redis" or "memory"
    redis: Option<RedisConfig>,
    memory: Option<MemoryCacheConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RedisConfig {
    host: String,
    port: u16,
    password: Option<String>,
    #[serde(default)]
    database: u32,
    #[serde(default = "default_redis_pool_size")]
    pool_size: u32,
}

#[derive(Debug, Deserialize, Serialize)]
struct MemoryCacheConfig {
    #[serde(default = "default_memory_cache_size")]
    max_size: u64,
    #[serde(default = "default_memory_cache_ttl")]
    default_ttl: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct MessagingConfig {
    #[serde(default = "default_messaging_type")]
    messaging_type: String, // "kafka", "rabbitmq", "nats"
    kafka: Option<KafkaConfig>,
    rabbitmq: Option<RabbitMQConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
struct KafkaConfig {
    brokers: Vec<String>,
    #[serde(default = "default_consumer_group")]
    consumer_group: String,
    topics: KafkaTopics,
}

#[derive(Debug, Deserialize, Serialize)]
struct KafkaTopics {
    events: String,
    commands: String,
    dead_letter: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct RabbitMQConfig {
    host: String,
    port: u16,
    username: String,
    password: String,
    vhost: String,
    exchanges: RabbitMQExchanges,
}

#[derive(Debug, Deserialize, Serialize)]
struct RabbitMQExchanges {
    events: String,
    commands: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ObservabilityConfig {
    logging: LoggingConfig,
    metrics: MetricsConfig,
    tracing: TracingConfig,
}

#[derive(Debug, Deserialize, Serialize)]
struct LoggingConfig {
    #[serde(default = "default_log_level")]
    level: String,
    #[serde(default = "default_log_format")]
    format: String,
    structured: bool,
    correlation_id: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct MetricsConfig {
    enabled: bool,
    #[serde(default = "default_metrics_port")]
    port: u16,
    #[serde(default = "default_metrics_path")]
    path: String,
    #[serde(default)]
    custom_metrics: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TracingConfig {
    enabled: bool,
    #[serde(default = "default_tracing_endpoint")]
    endpoint: String,
    #[serde(default = "default_sampling_rate")]
    sampling_rate: f64,
    service_name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct SecurityConfig {
    #[serde(default)]
    jwt: JwtConfig,
    #[serde(default)]
    cors: CorsConfig,
    #[serde(default)]
    rate_limiting: RateLimitConfig,
}

#[derive(Debug, Deserialize, Serialize)]
struct JwtConfig {
    #[serde(default)]
    enabled: bool,
    secret: Option<String>,
    #[serde(default = "default_jwt_expiry")]
    expiry: u64,
    issuer: Option<String>,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            secret: None,
            expiry: default_jwt_expiry(),
            issuer: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct CorsConfig {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    allowed_origins: Vec<String>,
    #[serde(default)]
    allowed_methods: Vec<String>,
    #[serde(default)]
    allowed_headers: Vec<String>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec!["GET".to_string(), "POST".to_string()],
            allowed_headers: vec!["Content-Type".to_string()],
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct RateLimitConfig {
    #[serde(default)]
    enabled: bool,
    #[serde(default = "default_requests_per_minute")]
    requests_per_minute: u32,
    #[serde(default = "default_burst_size")]
    burst_size: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            requests_per_minute: default_requests_per_minute(),
            burst_size: default_burst_size(),
        }
    }
}

// Default value functions
fn default_environment() -> String {
    "development".to_string()
}
fn default_instance_id() -> String {
    format!("instance-{}", std::process::id())
}
fn default_host() -> String {
    "0.0.0.0".to_string()
}
fn default_port() -> u16 {
    8080
}
fn default_shutdown_timeout() -> u64 {
    30
}
fn default_health_path() -> String {
    "/health".to_string()
}
fn default_health_interval() -> u64 {
    30
}
fn default_health_timeout() -> u64 {
    5
}
fn default_pool_size() -> u32 {
    10
}
fn default_max_idle() -> u32 {
    5
}
fn default_connection_timeout() -> u64 {
    30
}
fn default_cache_type() -> String {
    "memory".to_string()
}
fn default_redis_pool_size() -> u32 {
    10
}
fn default_memory_cache_size() -> u64 {
    100 * 1024 * 1024
} // 100MB
fn default_memory_cache_ttl() -> u64 {
    3600
} // 1 hour
fn default_messaging_type() -> String {
    "kafka".to_string()
}
fn default_consumer_group() -> String {
    "microservice-group".to_string()
}
fn default_log_level() -> String {
    "info".to_string()
}
fn default_log_format() -> String {
    "json".to_string()
}
fn default_metrics_port() -> u16 {
    9090
}
fn default_metrics_path() -> String {
    "/metrics".to_string()
}
fn default_tracing_endpoint() -> String {
    "http://localhost:14268/api/traces".to_string()
}
fn default_sampling_rate() -> f64 {
    0.1
}
fn default_jwt_expiry() -> u64 {
    3600
} // 1 hour
fn default_requests_per_minute() -> u32 {
    100
}
fn default_burst_size() -> u32 {
    10
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Microservice Configuration Example");
    println!("====================================");

    let mut spice_instance = setup_configuration()?;

    // Load the complete configuration
    let config: MicroserviceConfig = spice_instance.unmarshal()?;

    // Display configuration summary
    display_config_summary(&config);

    // Demonstrate configuration validation
    validate_configuration(&config)?;

    // Show environment-specific overrides
    demonstrate_environment_overrides(&spice_instance)?;

    // Demonstrate feature flag usage
    demonstrate_feature_flags(&config);

    println!("\n✅ Microservice configuration example completed!");
    Ok(())
}

fn setup_configuration() -> Result<Spice, Box<dyn std::error::Error>> {
    let mut spice_instance = Spice::new();

    println!("\n📋 Setting up microservice configuration...");

    // Set up defaults
    setup_service_defaults(&mut spice_instance)?;

    // Configure file search paths
    spice_instance.add_config_path(".");
    spice_instance.add_config_path("./config");
    spice_instance.add_config_path("/etc/microservice");

    // Try to load base configuration
    spice_instance.set_config_name("microservice");
    match spice_instance.read_in_config() {
        Ok(()) => println!("   ✓ Loaded microservice configuration"),
        Err(_) => {
            println!("   ⚠ No configuration file found, creating sample...");
            create_sample_microservice_config()?;
        }
    }

    // Set up environment variables with service prefix
    let service_name = env::var("SERVICE_NAME").unwrap_or_else(|_| "MICROSERVICE".to_string());
    let env_layer = EnvConfigLayer::new(Some(service_name), true);
    spice_instance.add_layer(Box::new(env_layer));

    println!("   ✓ Environment variables configured with prefix");

    Ok(spice_instance)
}

fn setup_service_defaults(spice_instance: &mut Spice) -> Result<(), Box<dyn std::error::Error>> {
    // Service defaults
    spice_instance.set_default("service.name", ConfigValue::from("my-microservice"))?;
    spice_instance.set_default("service.version", ConfigValue::from("1.0.0"))?;
    spice_instance.set_default("service.environment", ConfigValue::from("development"))?;

    // Server defaults
    spice_instance.set_default("server.host", ConfigValue::from("0.0.0.0"))?;
    spice_instance.set_default("server.port", ConfigValue::from(8080i64))?;
    spice_instance.set_default("server.shutdown_timeout", ConfigValue::from(30i64))?;
    spice_instance.set_default("server.health_check.path", ConfigValue::from("/health"))?;
    spice_instance.set_default("server.health_check.interval", ConfigValue::from(30i64))?;
    spice_instance.set_default("server.health_check.timeout", ConfigValue::from(5i64))?;

    // Database defaults
    spice_instance.set_default("database.host", ConfigValue::from("localhost"))?;
    spice_instance.set_default("database.port", ConfigValue::from(5432i64))?;
    spice_instance.set_default("database.database", ConfigValue::from("microservice"))?;
    spice_instance.set_default("database.username", ConfigValue::from("postgres"))?;
    spice_instance.set_default("database.password", ConfigValue::from("password"))?;
    spice_instance.set_default("database.pool_size", ConfigValue::from(10i64))?;

    // Cache defaults
    spice_instance.set_default("cache.cache_type", ConfigValue::from("memory"))?;
    spice_instance.set_default("cache.memory.max_size", ConfigValue::from(104857600i64))?; // 100MB
    spice_instance.set_default("cache.memory.default_ttl", ConfigValue::from(3600i64))?;

    // Messaging defaults
    spice_instance.set_default("messaging.messaging_type", ConfigValue::from("kafka"))?;

    // Observability defaults
    spice_instance.set_default("observability.logging.level", ConfigValue::from("info"))?;
    spice_instance.set_default("observability.logging.format", ConfigValue::from("json"))?;
    spice_instance.set_default("observability.logging.structured", ConfigValue::from(true))?;
    spice_instance.set_default(
        "observability.logging.correlation_id",
        ConfigValue::from(true),
    )?;

    spice_instance.set_default("observability.metrics.enabled", ConfigValue::from(true))?;
    spice_instance.set_default("observability.metrics.port", ConfigValue::from(9090i64))?;
    spice_instance.set_default("observability.metrics.path", ConfigValue::from("/metrics"))?;

    spice_instance.set_default("observability.tracing.enabled", ConfigValue::from(false))?;
    spice_instance.set_default(
        "observability.tracing.sampling_rate",
        ConfigValue::from(0.1),
    )?;

    // Security defaults
    spice_instance.set_default("security.jwt.enabled", ConfigValue::from(false))?;
    spice_instance.set_default("security.jwt.expiry", ConfigValue::from(3600i64))?;
    spice_instance.set_default("security.cors.enabled", ConfigValue::from(false))?;
    spice_instance.set_default("security.rate_limiting.enabled", ConfigValue::from(false))?;
    spice_instance.set_default(
        "security.rate_limiting.requests_per_minute",
        ConfigValue::from(100i64),
    )?;

    // Feature flags
    spice_instance.set_default("feature_flags.async_processing", ConfigValue::from(true))?;
    spice_instance.set_default("feature_flags.circuit_breaker", ConfigValue::from(false))?;
    spice_instance.set_default("feature_flags.request_tracing", ConfigValue::from(true))?;

    Ok(())
}

fn display_config_summary(config: &MicroserviceConfig) {
    println!("\n🔧 Microservice Configuration Summary:");
    println!("=====================================");

    println!(
        "Service: {} v{} ({})",
        config.service.name, config.service.version, config.service.environment
    );
    println!("Instance ID: {}", config.service.instance_id);

    println!("\nServer: {}:{}", config.server.host, config.server.port);
    println!(
        "Health Check: {} (every {}s)",
        config.server.health_check.path, config.server.health_check.interval
    );

    println!(
        "\nDatabase: {}:{}/{}",
        config.database.host, config.database.port, config.database.database
    );
    println!("Pool Size: {}", config.database.pool_size);

    println!("\nCache: {}", config.cache.cache_type);
    match config.cache.cache_type.as_str() {
        "redis" => {
            if let Some(redis) = &config.cache.redis {
                println!("  Redis: {}:{}", redis.host, redis.port);
            }
        }
        "memory" => {
            if let Some(memory) = &config.cache.memory {
                println!(
                    "  Memory: {} bytes, TTL: {}s",
                    memory.max_size, memory.default_ttl
                );
            }
        }
        _ => {}
    }

    println!("\nMessaging: {}", config.messaging.messaging_type);

    println!("\nObservability:");
    println!(
        "  Logging: {} ({})",
        config.observability.logging.level, config.observability.logging.format
    );
    println!(
        "  Metrics: {} on port {}",
        if config.observability.metrics.enabled {
            "enabled"
        } else {
            "disabled"
        },
        config.observability.metrics.port
    );
    println!(
        "  Tracing: {}",
        if config.observability.tracing.enabled {
            "enabled"
        } else {
            "disabled"
        }
    );

    println!("\nSecurity:");
    println!(
        "  JWT: {}",
        if config.security.jwt.enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!(
        "  CORS: {}",
        if config.security.cors.enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!(
        "  Rate Limiting: {}",
        if config.security.rate_limiting.enabled {
            "enabled"
        } else {
            "disabled"
        }
    );

    if !config.feature_flags.is_empty() {
        println!("\nFeature Flags:");
        for (flag, enabled) in &config.feature_flags {
            println!("  {}: {}", flag, enabled);
        }
    }
}

fn validate_configuration(config: &MicroserviceConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n✅ Validating configuration...");

    // Validate required fields
    if config.service.name.is_empty() {
        return Err("Service name cannot be empty".into());
    }

    if config.database.password.is_empty() {
        println!("   ⚠ Warning: Database password is empty");
    }

    // Validate port ranges
    if config.server.port < 1024 {
        println!(
            "   ⚠ Warning: Server port {} is in privileged range",
            config.server.port
        );
    }

    // Validate cache configuration
    match config.cache.cache_type.as_str() {
        "redis" => {
            if config.cache.redis.is_none() {
                return Err("Redis cache type selected but no Redis configuration provided".into());
            }
        }
        "memory" => {
            if config.cache.memory.is_none() {
                return Err(
                    "Memory cache type selected but no memory configuration provided".into(),
                );
            }
        }
        _ => {
            return Err(format!("Unknown cache type: {}", config.cache.cache_type).into());
        }
    }

    // Validate security configuration
    if config.security.jwt.enabled && config.security.jwt.secret.is_none() {
        return Err("JWT is enabled but no secret is configured".into());
    }

    println!("   ✓ Configuration validation passed");
    Ok(())
}

fn demonstrate_environment_overrides(
    spice_instance: &Spice,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🌍 Environment Override Examples:");
    println!("=================================");

    println!("Set these environment variables to override configuration:");
    println!("  export MICROSERVICE_SERVER_PORT=9000");
    println!("  export MICROSERVICE_DATABASE_HOST=prod-db.example.com");
    println!("  export MICROSERVICE_OBSERVABILITY_LOGGING_LEVEL=debug");
    println!("  export MICROSERVICE_SECURITY_JWT_ENABLED=true");
    println!("  export MICROSERVICE_FEATURE_FLAGS_CIRCUIT_BREAKER=true");

    // Show current values that could be overridden
    if let Some(port) = spice_instance.get_int("server.port")? {
        println!("\nCurrent server.port: {}", port);
    }

    if let Some(db_host) = spice_instance.get_string("database.host")? {
        println!("Current database.host: {}", db_host);
    }

    if let Some(log_level) = spice_instance.get_string("observability.logging.level")? {
        println!("Current logging level: {}", log_level);
    }

    Ok(())
}

fn demonstrate_feature_flags(config: &MicroserviceConfig) {
    println!("\n🚩 Feature Flag Usage:");
    println!("======================");

    for (flag, enabled) in &config.feature_flags {
        println!(
            "Feature '{}': {}",
            flag,
            if *enabled { "ON" } else { "OFF" }
        );

        // Demonstrate how you might use these in code
        match flag.as_str() {
            "async_processing" => {
                if *enabled {
                    println!("  → Async message processing is enabled");
                } else {
                    println!("  → Using synchronous processing");
                }
            }
            "circuit_breaker" => {
                if *enabled {
                    println!("  → Circuit breaker pattern is active");
                } else {
                    println!("  → Direct service calls (no circuit breaker)");
                }
            }
            "request_tracing" => {
                if *enabled {
                    println!("  → Request tracing and correlation IDs enabled");
                } else {
                    println!("  → Basic logging without request tracing");
                }
            }
            _ => {}
        }
    }
}

fn create_sample_microservice_config() -> Result<(), Box<dyn std::error::Error>> {
    let sample_config = r#"{
  "service": {
    "name": "user-service",
    "version": "2.1.0",
    "environment": "production",
    "tags": ["user-management", "authentication"]
  },
  "server": {
    "host": "0.0.0.0",
    "port": 8080,
    "shutdown_timeout": 30,
    "health_check": {
      "path": "/health",
      "interval": 30,
      "timeout": 5
    }
  },
  "database": {
    "host": "postgres.internal",
    "port": 5432,
    "database": "users",
    "username": "app_user",
    "password": "secure_password_123",
    "pool_size": 20,
    "max_idle": 10,
    "connection_timeout": 30
  },
  "cache": {
    "cache_type": "redis",
    "redis": {
      "host": "redis.internal",
      "port": 6379,
      "database": 0,
      "pool_size": 15
    }
  },
  "messaging": {
    "messaging_type": "kafka",
    "kafka": {
      "brokers": ["kafka1.internal:9092", "kafka2.internal:9092"],
      "consumer_group": "user-service-group",
      "topics": {
        "events": "user.events",
        "commands": "user.commands",
        "dead_letter": "user.dead_letter"
      }
    }
  },
  "observability": {
    "logging": {
      "level": "info",
      "format": "json",
      "structured": true,
      "correlation_id": true
    },
    "metrics": {
      "enabled": true,
      "port": 9090,
      "path": "/metrics",
      "custom_metrics": ["user_registrations", "login_attempts"]
    },
    "tracing": {
      "enabled": true,
      "endpoint": "http://jaeger:14268/api/traces",
      "sampling_rate": 0.1,
      "service_name": "user-service"
    }
  },
  "security": {
    "jwt": {
      "enabled": true,
      "secret": "your-secret-key-here",
      "expiry": 3600,
      "issuer": "user-service"
    },
    "cors": {
      "enabled": true,
      "allowed_origins": ["https://app.example.com"],
      "allowed_methods": ["GET", "POST", "PUT", "DELETE"],
      "allowed_headers": ["Content-Type", "Authorization"]
    },
    "rate_limiting": {
      "enabled": true,
      "requests_per_minute": 1000,
      "burst_size": 50
    }
  },
  "feature_flags": {
    "async_processing": true,
    "circuit_breaker": true,
    "request_tracing": true,
    "new_user_flow": false,
    "enhanced_security": true
  }
}"#;

    std::fs::write("microservice.json", sample_config)?;
    println!("   ✓ Created sample microservice.json");
    Ok(())
}
