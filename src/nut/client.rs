// Licensed under the Open Software License version 3.0
#[mockall_double::double]
use super::connection::Connection;
use super::{config::NetworkUpsToolsClientConfig, sender::UninterruptiblePowerSupplyData};
use crate::hardware::types::{HardwareMetadata, HardwareType, SourceType};
use rups::Config;
use serde::{Deserialize, Serialize};
use std::{cmp::min, collections::HashMap, sync::Arc, time::Duration};
use tokio::{
    sync::{Mutex, RwLock},
    time::sleep,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UninterruptiblePowerSupply {
    pub meta: HardwareMetadata,
    ups_name: String,
    variables_to_monitor: Vec<String>,
}
impl UninterruptiblePowerSupply {
    pub fn new(
        ups_name: String,
        server_id: String,
        variables_to_monitor: Option<Vec<String>>,
    ) -> Self {
        // Create id by prepending "[ups_name]" to "server_id"
        let id = format!("[{}]{}", ups_name, server_id);
        let mut variables_to_monitor = variables_to_monitor.unwrap_or_default();
        // Warn if there are no variables to monitor
        if variables_to_monitor.is_empty() {
            tracing::warn!("No variables to monitor for UPS {}, using defaults", id);
            variables_to_monitor = vec![
                String::from("battery.charge"),
                String::from("battery.charge.low"),
                String::from("battery.runtime"),
                String::from("battery.runtime.low"),
                String::from("input.frequency"),
                String::from("input.voltage"),
                String::from("output.frequency"),
                String::from("output.frequency.nominal"),
                String::from("output.voltage"),
                String::from("output.voltage.nominal"),
                String::from("ups.load"),
                String::from("ups.power"),
                String::from("ups.power.nominal"),
                String::from("ups.realpower"),
                String::from("ups.status"),
            ];
        }
        Self {
            meta: HardwareMetadata::new(
                id,
                HardwareType::UninterruptiblePowerSupply,
                SourceType::NetworkUpsTools,
            ),
            ups_name,
            variables_to_monitor,
        }
    }

    fn get_variables_to_monitor(&self) -> Vec<String> {
        self.variables_to_monitor.clone()
    }

    pub async fn query_variables(
        &self,
        guarded_connection: Arc<Mutex<Option<Connection>>>,
    ) -> HashMap<String, String> {
        let mut variables_with_values: HashMap<String, String> = HashMap::new();
        // Acquire lock on connection
        let mut locked_connection = guarded_connection.lock().await;
        // Borrow connection
        let connection = locked_connection.take();
        // Return empty hashmap if not connected
        if connection.is_none() {
            return variables_with_values;
        }
        // Unwrap connection and query server for variables
        let mut connection = connection.unwrap();
        for variable_to_get in self.get_variables_to_monitor() {
            let returned_variable = connection
                .get_var(&self.ups_name, &variable_to_get)
                .await
                .ok();
            if returned_variable.is_some() {
                variables_with_values.insert(variable_to_get, returned_variable.unwrap().value());
            } else {
                tracing::warn!(
                    "Failed to get variable {} from UPS {}",
                    variable_to_get,
                    self.meta.hw.id
                )
            }
        }
        // Release connection
        locked_connection.replace(connection);
        // Return variables as key-value hashmap
        variables_with_values
    }
}

pub struct NetworkUpsToolsClient {
    // Required to do basic tasks
    connection: Arc<Mutex<Option<Connection>>>,
    upses: Vec<UninterruptiblePowerSupply>,
    // Required to (re)connect
    rups_config: Config,
    failed_attempts: Arc<RwLock<u32>>,
    cooldown: Duration,
    // Required for tracing
    server_id: String,
}

impl NetworkUpsToolsClient {
    // Easy to construct from deserialized config
    pub fn new(client_config: &NetworkUpsToolsClientConfig, cooldown: Duration) -> Self {
        let server_id = client_config.get_server_id();
        let rups_config = client_config.build_rups_config();
        let upses = client_config.get_upses(server_id.clone());

        Self {
            connection: Arc::new(Mutex::new(None)),
            upses,
            rups_config,
            failed_attempts: Arc::new(RwLock::new(0)),
            cooldown,
            server_id,
        }
    }

    async fn is_connected(&self) -> bool {
        let mut locked_connection = self.connection.lock().await;
        let connection = locked_connection.take();
        if let Some(mut conn) = connection {
            // Send a request to check connection
            if conn.get_server_version().await.is_ok() {
                locked_connection.replace(conn);
                return true;
            }
        }
        false
    }

    async fn connect(&self) {
        // Acquire locks
        let mut locked_connection = self.connection.lock().await;
        let mut locked_failed_attempts = self.failed_attempts.write().await;
        *locked_failed_attempts = locked_failed_attempts.saturating_add(1);
        // Try to connect
        let connection = Connection::new(&self.rups_config).await;
        // Handle failure
        if connection.is_err() {
            let error_message = connection.err().unwrap();
            tracing::warn!(
                "Failed to connect to UPS {}: {:?}",
                self.server_id,
                error_message
            );
            return;
        }
        // On success: reset failed attempts and save connection
        let connection = connection.unwrap();
        tracing::debug!("Connected to UPS {:?}", self.server_id);
        *locked_failed_attempts = 0;
        locked_connection.replace(connection);
    }

    async fn connect_if_not_connected(&self) {
        // Retry on errors, linear backoff (cooldown*failed_attempts)
        while !self.is_connected().await {
            let failed_attempts: u32;
            // Keep the .read() lock as short as possible to prevent deadlock
            {
                failed_attempts = *self.failed_attempts.read().await;
            }
            let should_sleep_for = self.cooldown.saturating_mul(failed_attempts);
            let sleep_for = min(should_sleep_for, Duration::from_secs(3600)); // Limit to 1 hour
            sleep(sleep_for).await;
            self.connect().await;
        }
    }

    pub async fn query_all_upses(&self) -> Vec<UninterruptiblePowerSupplyData> {
        // Check connection
        self.connect_if_not_connected().await;
        // Query all UPSes
        let mut data_from_upses: Vec<UninterruptiblePowerSupplyData> = Vec::new();
        for ups in &self.upses {
            let variables = ups.query_variables(self.connection.clone()).await;
            data_from_upses.push(UninterruptiblePowerSupplyData::new(ups, variables));
        }
        data_from_upses
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::Example;

    #[tokio::test]
    async fn test_connect() {
        let config = NetworkUpsToolsClientConfig::example();
        let cooldown = Duration::default();
        let client = NetworkUpsToolsClient::new(&config, cooldown);

        assert!(!client.is_connected().await);
        client.connect().await;
        assert!(client.is_connected().await);
    }

    #[tokio::test]
    async fn test_query_all_upses() {
        let config = NetworkUpsToolsClientConfig::example();
        let cooldown = Duration::default();
        let client = NetworkUpsToolsClient::new(&config, cooldown);
        let upses = client.query_all_upses().await;
        assert_eq!(upses.len(), 1);

        let ups = &upses[0];
        assert_eq!(ups.meta.hw.id, "[ups1]ups-monitor@localhost:3493");
        assert_eq!(
            ups.meta.hw.hardware_type,
            HardwareType::UninterruptiblePowerSupply
        );
        assert_eq!(ups.meta.source.source_type, SourceType::NetworkUpsTools);

        let variables = &ups.variables;
        assert_eq!(variables.len(), 4);
        assert_eq!(variables.get("battery.charge").unwrap(), "100");
        assert_eq!(variables.get("battery.charge.low").unwrap(), "30");
        assert_eq!(variables.get("battery.runtime").unwrap(), "15");
        assert_eq!(variables.get("battery.runtime.low").unwrap(), "5");
    }
}
