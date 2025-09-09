# Contributing to Spice

Thank you for your interest in contributing to Spice! This document provides guidelines and information for contributors.

## Code of Conduct

This project adheres to a code of conduct that we expect all contributors to follow. Please be respectful and constructive in all interactions.

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Git
- A GitHub account

### Development Setup

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/myself659/spicex.git
   cd spicex
   ```

3. Add the upstream repository:
   ```bash
   git remote add upstream https://github.com/myself659/spicex.git
   ```

4. Create a new branch for your feature or fix:
   ```bash
   git checkout -b feature/your-feature-name
   ```

5. Build and test the project:
   ```bash
   cargo build
   cargo test
   cargo test --features cli  # Test with CLI features
   ```

## Development Workflow

### Building

```bash
# Standard build
cargo build

# Build with all features
cargo build --all-features

# Release build
cargo build --release
```

### Testing

```bash
# Run all tests
cargo test

# Run tests with CLI features
cargo test --features cli

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run integration tests
cargo test --test integration_tests
```

### Documentation

```bash
# Generate documentation
cargo doc --open

# Test documentation examples
cargo test --doc

# Check documentation coverage
cargo doc --document-private-items
```

### Linting and Formatting

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --check

# Run clippy lints
cargo clippy

# Run clippy with all features
cargo clippy --all-features

# Fix clippy suggestions automatically
cargo clippy --fix
```

### Running Examples

```bash
# Basic usage example
cargo run --example basic_usage

# CLI example with features
cargo run --example cli_flag_usage --features cli

# File watching example
echo '{"debug": true}' > config.json
cargo run --example file_watching
```

## Contribution Guidelines

### Pull Request Process

1. **Create an Issue First**: For significant changes, create an issue to discuss the proposed changes before starting work.

2. **Keep Changes Focused**: Each pull request should address a single concern. Avoid mixing unrelated changes.

3. **Write Tests**: All new functionality should include comprehensive tests. Aim for high test coverage.

4. **Update Documentation**: Update relevant documentation, including:
   - API documentation (rustdoc comments)
   - README.md if adding new features
   - Examples if demonstrating new functionality

5. **Follow Coding Standards**: Ensure your code follows the project's coding standards (see below).

6. **Write Clear Commit Messages**: Use descriptive commit messages that explain what and why, not just what.

### Coding Standards

#### Code Style

- Follow standard Rust formatting (`cargo fmt`)
- Use `cargo clippy` and address all warnings
- Prefer explicit types when it improves readability
- Use meaningful variable and function names
- Keep functions focused and reasonably sized

#### Documentation

- All public APIs must have rustdoc comments
- Include examples in documentation when helpful
- Document error conditions and edge cases
- Use proper markdown formatting in documentation

#### Error Handling

- Use the `ConfigResult<T>` type alias for consistency
- Provide descriptive error messages
- Use appropriate error variants from `ConfigError`
- Preserve error context when propagating errors

#### Testing

- Write unit tests for all new functionality
- Include integration tests for complex features
- Test error conditions and edge cases
- Use descriptive test names that explain what is being tested

### Code Organization

#### Module Structure

```
src/
├── lib.rs              # Main library entry point
├── config.rs           # Core Viper struct and implementation
├── value.rs            # ConfigValue type and conversions
├── error.rs            # Error types and utilities
├── layer.rs            # Configuration layer abstractions
├── parser.rs           # File format parsers
├── file_layer.rs       # File-based configuration layer
├── env_layer.rs        # Environment variable layer
├── default_layer.rs    # Default values layer
├── watcher.rs          # File watching utilities
└── cli.rs              # Command line flag support (optional)
```

#### Naming Conventions

- Use `snake_case` for functions, variables, and modules
- Use `PascalCase` for types, structs, and enums
- Use `SCREAMING_SNAKE_CASE` for constants
- Prefix private items with underscore when appropriate

### Adding New Features

#### Configuration Sources

To add a new configuration source:

1. Create a new module (e.g., `src/remote_layer.rs`)
2. Implement the `ConfigLayer` trait
3. Add appropriate priority level to `LayerPriority` enum
4. Add integration methods to the main `Viper` struct
5. Write comprehensive tests
6. Update documentation and examples

#### File Format Parsers

To add a new file format parser:

1. Add parser struct to `src/parser.rs`
2. Implement the `ConfigParser` trait
3. Add format detection to `detect_parser_by_extension`
4. Add conversion functions between format and `ConfigValue`
5. Write comprehensive tests including edge cases
6. Update documentation with format examples

#### Configuration Methods

When adding new configuration access methods:

1. Follow existing naming patterns (`get_*`, `set_*`)
2. Provide both fallible and infallible variants when appropriate
3. Include comprehensive rustdoc with examples
4. Add corresponding tests
5. Consider type safety and ergonomics

### Testing Guidelines

#### Unit Tests

- Test each function/method in isolation
- Use descriptive test names: `test_get_string_with_valid_key`
- Test both success and failure cases
- Use `assert_eq!` for exact matches, `assert!` for boolean conditions
- Group related tests in modules

#### Integration Tests

- Test complete workflows and feature interactions
- Use temporary files/directories for file system tests
- Clean up resources in tests (use `tempfile` crate)
- Test configuration precedence and layer interactions

#### Test Organization

```rust
#[cfg(test)]
mod tests {
    use super::*;

    mod unit_tests {
        use super::*;

        #[test]
        fn test_specific_functionality() {
            // Test implementation
        }
    }

    mod integration_tests {
        use super::*;

        #[test]
        fn test_complete_workflow() {
            // Integration test implementation
        }
    }
}
```

### Documentation Guidelines

#### API Documentation

- Document all public items with rustdoc comments
- Include at least one example for complex functions
- Document parameters, return values, and errors
- Use proper markdown formatting

```rust
/// Retrieves a configuration value as a string.
///
/// This method searches through all configuration layers in precedence order
/// and returns the first string value found for the given key.
///
/// # Arguments
/// * `key` - The configuration key to retrieve (supports dot notation)
///
/// # Returns
/// * `ConfigResult<Option<String>>` - The string value if found, None if not found
///
/// # Errors
/// * `ConfigError::TypeConversion` - If the value exists but cannot be converted to string
///
/// # Example
/// ```rust
/// use viper_rust::{Viper, ConfigValue};
///
/// let mut viper = Viper::new();
/// viper.set("database.host", ConfigValue::from("localhost")).unwrap();
///
/// let host = viper.get_string("database.host").unwrap();
/// assert_eq!(host, Some("localhost".to_string()));
/// ```
pub fn get_string(&self, key: &str) -> ConfigResult<Option<String>> {
    // Implementation
}
```

#### Examples

- Keep examples simple and focused
- Use realistic configuration scenarios
- Include error handling when relevant
- Ensure examples compile and run correctly

### Performance Considerations

- Avoid unnecessary allocations
- Use `Cow<str>` for string handling when appropriate
- Cache frequently accessed values
- Consider lazy loading for expensive operations
- Profile performance-critical code paths

### Security Considerations

- Validate all input from external sources
- Be careful with file system operations
- Avoid exposing sensitive information in error messages
- Consider security implications of configuration sources

## Release Process

### Version Numbering

This project follows [Semantic Versioning](https://semver.org/):

- **MAJOR**: Incompatible API changes
- **MINOR**: New functionality in a backwards compatible manner
- **PATCH**: Backwards compatible bug fixes

### Changelog

- Update `CHANGELOG.md` with all changes
- Group changes by type: Added, Changed, Deprecated, Removed, Fixed, Security
- Include migration notes for breaking changes

### Release Checklist

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Run full test suite
4. Update documentation
5. Create release PR
6. Tag release after merge
7. Publish to crates.io

## Getting Help

- **Issues**: Create a GitHub issue for bugs or feature requests
- **Discussions**: Use GitHub Discussions for questions and general discussion
- **Documentation**: Check the API documentation and examples
- **Code Review**: Don't hesitate to ask for feedback on your pull requests

## Recognition

Contributors will be recognized in:
- `CONTRIBUTORS.md` file
- Release notes for significant contributions
- GitHub contributors page

Thank you for contributing to Spice!