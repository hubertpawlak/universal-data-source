// Licensed under the Open Software License version 3.0
use crate::config::NetworkUpsToolsClientConfig;
use rups::{blocking::Connection, Auth, Config, ConfigBuilder};

pub struct NetworkUpsToolsClient {
    // Take ownership of the connection to use it
    connection: Option<Connection>,
    // Internal state
    server_id: String,
    config: Config,
    // Handle failed connections
    failed_attempts: u8,  // Max 255
    cooldown_counter: u8, // Don't try to connect for N loop iterations
}

impl NetworkUpsToolsClient {
    // Easy to construct from deserialized config
    pub fn new(config: &NetworkUpsToolsClientConfig) -> Self {
        let server_id = config.get_server_id();
        // Prepare authentication details
        let auth: Option<Auth> = match (&config.username, &config.password) {
            (Some(username), Some(password)) => {
                Some(Auth::new(username.clone(), Some(password.clone())))
            }
            _ => None,
        };
        // Build config and return it
        let config = ConfigBuilder::new()
            .with_host(
                (
                    config.host.clone(),
                    config.port.unwrap_or(rups::DEFAULT_PORT),
                )
                    .try_into()
                    .unwrap_or_default(),
            )
            .with_auth(auth)
            .with_ssl(config.enable_tls.unwrap_or(false))
            .build();
        Self {
            // Don't connect yet, wait for first request
            connection: None,
            server_id,
            config,
            failed_attempts: 0,
            cooldown_counter: 0,
        }
    }

    pub fn get_server_id(&self) -> String {
        self.server_id.clone()
    }

    pub fn take_connection(&mut self) -> Option<Connection> {
        // Create new connection if it doesn't exist
        // Log error if it fails
        if self.connection.is_none() {
            // Check if cooldown is active
            if self.cooldown_counter > 0 {
                // Safely decrement cooldown counter
                self.cooldown_counter = self.cooldown_counter.saturating_sub(1);
                // Don't try at least until next loop iteration
                return None;
            }
            match Connection::new(&self.config) {
                Ok(connection) => {
                    // Connection succeeded, reset failed attempts
                    self.failed_attempts = 0;
                    self.connection = Some(connection);
                    log::info!("{} - Connected", self.server_id);
                }
                Err(error) => {
                    // Connection failed, increment failed attempts and set cooldown period
                    self.failed_attempts = self.failed_attempts.saturating_add(1);
                    self.cooldown_counter = self.failed_attempts; // Simple linear backoff
                    log::warn!(
                        "{} - {} (waiting for {} loop iterations)",
                        self.server_id,
                        error,
                        self.failed_attempts
                    );
                }
            }
        }
        self.connection.take()
    }

    pub fn give_connection(&mut self, connection: Connection) {
        self.connection = Some(connection);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::UniversalPowerSupplyConfig;

    #[test]
    fn test_get_server_id_default() {
        let config = NetworkUpsToolsClientConfig {
            host: "localhost".to_string(),
            port: None,
            enable_tls: None,
            username: None,
            password: None,
            upses: vec![UniversalPowerSupplyConfig {
                name: "ups".to_string(),
                variables_to_monitor: None,
            }],
        };
        let client = NetworkUpsToolsClient::new(&config);
        assert_eq!(client.get_server_id(), "@localhost:3493");
    }

    #[test]
    fn test_get_server_id_custom() {
        let config = NetworkUpsToolsClientConfig {
            host: "localhost".to_string(),
            port: Some(1234),
            enable_tls: Some(false),
            username: Some("username".to_string()),
            password: Some("password".to_string()),
            upses: vec![UniversalPowerSupplyConfig {
                name: "ups".to_string(),
                variables_to_monitor: Some(vec!["var".to_string()]),
            }],
        };
        let client = NetworkUpsToolsClient::new(&config);
        assert_eq!(client.get_server_id(), "username@localhost:1234");
    }
}
