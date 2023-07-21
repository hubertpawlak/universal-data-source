// Licensed under the Open Software License version 3.0
use super::types::{Config, Example};
use serde::Serialize;
use serde_json::{ser::PrettyFormatter, Serializer};
use std::{
    fs::{self},
    path::PathBuf,
    process,
};

fn write_default_config_to_file(path: &PathBuf) -> bool {
    // Create default config
    let config = Config::example();
    // Use 4 spaces for indentation
    let formatter = PrettyFormatter::with_indent(b"    ");
    // Serialize config to pretty json
    let mut buffer = Vec::new();
    let mut serializer = Serializer::with_formatter(&mut buffer, formatter);
    config.serialize(&mut serializer).unwrap();
    let json = String::from_utf8(buffer).unwrap();
    // Write config to file and return result
    fs::write(path, json).is_ok()
}

///  Checks if config file exists and creates it if not
/// # Returns
/// `true` if config was written to file
/// `false` if config file already exists
fn create_default_config_if_not_exists(path: &PathBuf) -> bool {
    if !path.exists() {
        // Create default config
        return write_default_config_to_file(path);
    }
    false
}

fn read_config(path: &PathBuf) -> Result<Config, Box<dyn std::error::Error>> {
    // Try to read config file and pass error if failed
    let config_file = fs::read_to_string(path)?;
    // Try to parse config file and pass error if failed
    let config: Config = serde_json::from_str(&config_file)?;
    // Return config
    Ok(config)
}

pub fn read_config_or_create_default() -> Config {
    // Get path to config file from "UDS_RS_CONFIG_FILE" env var
    // If not set, use "config.json" in current directory
    tracing::trace!("Determining config file path");
    let config_file_path = std::env::var("UDS_RS_CONFIG_FILE")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("config.json"));
    tracing::debug!("Reading config from: {}", config_file_path.display());
    // Read config from file
    // Exit on failure
    let config = match read_config(&config_file_path) {
        Ok(config) => config,
        Err(error) => {
            tracing::error!("Failed to read config: {}", error);
            // Write default config to file
            if create_default_config_if_not_exists(&config_file_path) {
                tracing::error!(
                    "Wrote default config to {}. Please edit this file and try again.",
                    config_file_path.display()
                );
            } else {
                tracing::error!(
                    "Failed to create default config. Try manually deleting {} and running again",
                    config_file_path.display()
                );
            }
            process::exit(1);
        }
    };
    tracing::debug!("Successfully read config");
    config
}

#[cfg(test)]
// Use tempfile::TempDir
mod tests {
    use super::*;
    use crate::config::types::Example;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_write_default_config_to_file() {
        // Create temp dir
        let temp_dir = tempfile::tempdir().unwrap();
        // Create path to config file
        let config_file_path = temp_dir.path().join("config.json");
        // Write default config to file
        write_default_config_to_file(&config_file_path);
        // Check if file exists
        assert!(config_file_path.exists());
    }

    #[test]
    fn test_create_default_config_if_not_exists() {
        // Create temp dir
        let temp_dir = tempfile::tempdir().unwrap();
        // Create path to config file
        let config_file_path = temp_dir.path().join("config.json");
        // Create default config if not exists
        create_default_config_if_not_exists(&config_file_path);
        // Check if file exists
        assert!(config_file_path.exists());
        // Create default config if not exists again
        create_default_config_if_not_exists(&config_file_path);
        // Check if file still exists
        assert!(config_file_path.exists());
    }

    #[test]
    fn test_read_config() {
        // Create temp dir
        let temp_dir = tempfile::tempdir().unwrap();
        // Create path to config file
        let config_file_path = temp_dir.path().join("config.json");
        // Create default config
        let config = Config::example();
        // Serialize config to pretty json
        let json = serde_json::to_string_pretty(&config).unwrap();
        // Write config to file
        let mut file = File::create(&config_file_path).unwrap();
        file.write_all(json.as_bytes()).unwrap();
        // Read config from file
        let read_config = read_config(&config_file_path).unwrap();
        // Check if config is equal to default config
        assert_eq!(read_config, config);
    }
}
