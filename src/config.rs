// Licensed under the Open Software License version 3.0
use serde::{Deserialize, Serialize};
use serde_json::{ser::PrettyFormatter, Serializer};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Serialize, Deserialize)]
pub struct Endpoint {
    pub url: String,
    pub bearer_token: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UniversalPowerSupplyConfig {
    pub name: String,
    pub variables_to_monitor: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
pub struct NetworkUpsToolsClientConfig {
    pub host: String,
    pub port: Option<u16>,
    pub enable_tls: Option<bool>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub upses: Vec<UniversalPowerSupplyConfig>,
}

impl NetworkUpsToolsClientConfig {
    pub fn get_server_id(&self) -> String {
        // Format server id as username@host:port
        // It should be done here because it's used in multiple places
        format!(
            "{}@{}:{}",
            self.username.clone().unwrap_or_default(),
            self.host,
            self.port.unwrap_or(rups::DEFAULT_PORT),
        )
    }
}

#[derive(Serialize, Deserialize)]
/// `Config` struct for deserializing config.json.
/// Automatically generated file contains all fields
/// but only endpoints are required.
pub struct Config {
    // Required fields
    // Missing values cause deserialization to fail
    // and regenerate config.json if not exists
    pub endpoints: Vec<Endpoint>,
    // Optional fields
    // Can be safely unwrapped with unwrap_or_default()
    // send.rs
    pub send_interval: Option<Duration>,
    // ds18b20.rs
    pub enable_one_wire: Option<bool>,
    pub one_wire_path_prefix: Option<String>,
    // ups.rs and nut.rs
    pub enable_ups_monitoring: Option<bool>,
    pub nut_connections: Option<Vec<NetworkUpsToolsClientConfig>>,
}
impl ::std::default::Default for Config {
    fn default() -> Self {
        Self {
            endpoints: vec![
                Endpoint {
                    url: "http://localhost:3000".to_string(),
                    bearer_token: None,
                },
                Endpoint {
                    url: "http://localhost:3001".to_string(),
                    bearer_token: Some("YOUR_SECRET_TOKEN".to_string()),
                },
            ],
            send_interval: Some(Duration::from_secs(5)),
            enable_one_wire: Some(false),
            one_wire_path_prefix: Some(String::from("/sys/bus/w1/devices")),
            enable_ups_monitoring: Some(false),
            nut_connections: Some(vec![NetworkUpsToolsClientConfig {
                host: "ups.lan".to_string(),
                port: Some(rups::DEFAULT_PORT),
                enable_tls: Some(false),
                username: None,
                password: None,
                upses: vec![
                    UniversalPowerSupplyConfig {
                        name: "ups1".to_string(),
                        variables_to_monitor: None,
                    },
                    UniversalPowerSupplyConfig {
                        name: "ups2".to_string(),
                        variables_to_monitor: Some(vec![
                            "battery.charge".to_string(),
                            "battery.runtime".to_string(),
                        ]),
                    },
                ],
            }]),
        }
    }
}

pub fn write_default_config_to_file(path: &PathBuf) -> bool {
    // Create default config
    let config = Config::default();
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
pub fn create_default_config_if_not_exists(path: &PathBuf) -> bool {
    if !Path::new(path).exists() {
        // Create default config
        return write_default_config_to_file(path);
    }
    false
}

pub fn read_config(path: &PathBuf) -> Result<Config, Box<dyn std::error::Error>> {
    // Try to read config file
    let config_file = fs::read_to_string(path);
    // Pass error if read failed
    let config_file = config_file?;
    // Try to parse config file and pass error if failed
    let config: Config = serde_json::from_str(&config_file)?;
    // Return config
    Ok(config)
}

#[cfg(test)]
// Use tempfile::TempDir
mod tests {
    use super::*;
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
        let config = Config::default();
        // Serialize config to pretty json
        let json = serde_json::to_string_pretty(&config).unwrap();
        // Write config to file
        let mut file = File::create(&config_file_path).unwrap();
        file.write_all(json.as_bytes()).unwrap();
        // Read config from file
        let read_config = read_config(&config_file_path).unwrap();
        // Check if config is equal to default config
        // Deep equal check is not possible because of Duration
        assert_eq!(read_config.endpoints.len(), config.endpoints.len());
        assert_eq!(
            read_config.one_wire_path_prefix,
            config.one_wire_path_prefix
        );
        assert_eq!(read_config.send_interval, config.send_interval);
        assert_eq!(read_config.enable_one_wire, config.enable_one_wire);
        assert_eq!(
            read_config.enable_ups_monitoring,
            config.enable_ups_monitoring
        );
        assert_eq!(
            read_config.nut_connections.unwrap().len(),
            config.nut_connections.unwrap().len()
        );
    }
}
