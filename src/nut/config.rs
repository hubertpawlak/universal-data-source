// Licensed under the Open Software License version 3.0
use super::client::UninterruptiblePowerSupply;
use crate::config::types::Example;
use rups::{Auth, Config, ConfigBuilder};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UninterruptiblePowerSupplyConfig {
    pub name: String,
    pub variables_to_monitor: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkUpsToolsClientConfig {
    host: String,
    port: Option<u16>,
    enable_tls: Option<bool>,
    username: Option<String>,
    password: Option<String>,
    upses: Vec<UninterruptiblePowerSupplyConfig>,
}

impl Example for NetworkUpsToolsClientConfig {
    fn example() -> Self {
        Self {
            host: String::from("localhost"),
            port: Some(rups::DEFAULT_PORT),
            enable_tls: Some(false),
            username: Some(String::from("ups-monitor")),
            password: Some(String::from("EXAMPLE_PASSWORD")),
            upses: vec![UninterruptiblePowerSupplyConfig {
                name: String::from("ups1"),
                variables_to_monitor: Some(vec![
                    String::from("battery.charge"),
                    String::from("battery.charge.low"),
                    String::from("battery.runtime"),
                    String::from("battery.runtime.low"),
                ]),
            }],
        }
    }
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

    pub fn build_rups_config(&self) -> Config {
        // Read-only commands don't need auth
        let auth: Option<Auth> = match (self.username.clone(), self.password.clone()) {
            (Some(username), Some(password)) => Some(Auth::new(username, Some(password))),
            _ => None,
        };

        ConfigBuilder::new()
            .with_timeout(Duration::from_secs(1))
            .with_host(
                (self.host.clone(), self.port.unwrap_or(rups::DEFAULT_PORT))
                    .try_into()
                    .unwrap_or_default(),
            )
            .with_auth(auth)
            .with_ssl(self.enable_tls.unwrap_or(false))
            .build()
    }

    pub fn get_upses(&self, server_id: String) -> Vec<UninterruptiblePowerSupply> {
        self.upses
            .iter()
            .map(|config| {
                UninterruptiblePowerSupply::new(
                    config.name.clone(),
                    server_id.clone(),
                    config.variables_to_monitor.clone(),
                )
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct UpsMonitoringConfig {
    enabled: Option<bool>,
    servers: Option<Vec<NetworkUpsToolsClientConfig>>,
    cooldown: Option<Duration>,
}

impl Example for UpsMonitoringConfig {
    fn example() -> Self {
        Self {
            enabled: Some(true),
            cooldown: Some(Duration::from_secs(5)),
            servers: Some(vec![NetworkUpsToolsClientConfig::example()]),
        }
    }
}

impl UpsMonitoringConfig {
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or_default()
    }

    pub fn get_server_configs(&self) -> Vec<NetworkUpsToolsClientConfig> {
        self.servers.clone().unwrap_or_default()
    }

    pub fn get_cooldown(&self) -> Duration {
        self.cooldown.unwrap_or(Duration::from_secs(5))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_config_get_server_id() {
        let config = NetworkUpsToolsClientConfig::example();
        assert_eq!(config.get_server_id(), "ups-monitor@localhost:3493");
    }
}
