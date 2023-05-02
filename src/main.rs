// Licensed under the Open Software License version 3.0
use crate::config::create_default_config_if_not_exists;
use config::NetworkUpsToolsClientConfig;
use ds18b20::MeasuredTemperature;
use nut::NetworkUpsToolsClient;
use send::DataToSend;
use std::{
    cmp,
    collections::HashMap,
    path::PathBuf,
    process, thread,
    time::{Duration, Instant},
};
use ups::{UninterruptiblePowerSupply, UninterruptiblePowerSupplyData};
mod config;
mod ds18b20;
mod hardware;
mod nut;
mod scanner;
mod send;
mod ups;

fn main() {
    // Initialize logger
    env_logger::init();
    // Get path to config file from "UDS_RS_CONFIG_FILE" env var
    // If not set, use "config.json" in current directory
    let config_file_path = match std::env::var("UDS_RS_CONFIG_FILE") {
        Ok(path) => PathBuf::from(path),
        Err(_) => PathBuf::from("config.json"),
    };
    // Read config from file
    // Exit on failure
    let config = match config::read_config(&config_file_path) {
        Ok(config) => config,
        Err(error) => {
            log::error!("Failed to read config: {}", error);
            // Write default config to file
            if create_default_config_if_not_exists(&config_file_path) {
                log::error!(
                    "Wrote default config to {}. Please edit this file and try again.",
                    config_file_path.display()
                );
            } else {
                log::error!(
                    "Failed to create default config. Try manually deleting {} and running again",
                    config_file_path.display()
                );
            }
            process::exit(1);
        }
    };
    // Extract useful values from config to avoid cloning inside loop
    let one_wire_path_prefix: PathBuf =
        PathBuf::from(&config.one_wire_path_prefix.unwrap_or_default());
    let send_interval = &config.send_interval.unwrap_or_default();
    let ignore_connection_errors = &config.ignore_connection_errors.unwrap_or_default();
    let enable_one_wire = &config.enable_one_wire.unwrap_or_default();
    let enable_ups_monitoring = &config.enable_ups_monitoring.unwrap_or_default();
    // Warn if endpoints are empty
    if config.endpoints.is_empty() {
        log::warn!("No endpoints configured. No data will be sent.");
    }
    // Build config for each NUT server
    let nut_configs: Vec<NetworkUpsToolsClientConfig> = match enable_ups_monitoring {
        true => config
            .nut_connections
            .unwrap_or_default()
            .into_iter()
            .collect(),
        // Return empty vector if UPS monitoring is disabled
        false => Vec::new(),
    };
    // Create a Vec of UninterruptiblePowerSupply structs
    let upses: Vec<UninterruptiblePowerSupply> = match enable_ups_monitoring {
        true => nut_configs
            .iter()
            .flat_map(|client_config| {
                // Return all UPSes for this server
                client_config
                    .upses
                    .clone()
                    .into_iter()
                    .map(|ups_config| -> UninterruptiblePowerSupply {
                        UninterruptiblePowerSupply::new(
                            client_config.get_server_id(),
                            ups_config.name,
                            ups_config.variables_to_monitor,
                        )
                    })
                    .collect::<Vec<UninterruptiblePowerSupply>>()
            })
            .collect(),
        // Return empty vector if UPS monitoring is disabled
        false => Vec::new(),
    };
    // Create a HashMap of connections by server_id
    let mut nut_clients: HashMap<String, NetworkUpsToolsClient> = match enable_ups_monitoring {
        true => nut_configs
            .into_iter()
            .map(|client_config| {
                // Create a new NUT client
                let client = NetworkUpsToolsClient::new(&client_config);
                // Return a tuple of (server_id, client)
                (client.get_server_id(), client)
            })
            .collect(),
        // Return empty HashMap if UPS monitoring is disabled
        false => HashMap::new(),
    };
    // Create a reusable reqwest client
    let reqwest_client = reqwest::blocking::Client::new();
    // Loop every send_interval seconds
    // Send data to all endpoints
    // Send data from all sensors
    loop {
        // Get start time
        let start_time = Instant::now();
        // Check if 1-Wire is enabled
        let sensors: Vec<MeasuredTemperature> = match enable_one_wire {
            true => {
                // Find all sensors - calling inside loop makes sensors hot-swappable
                let sensors = scanner::get_all_ds18b20_sensors(&one_wire_path_prefix);
                // Map additional fields: temperature and resolution
                let sensors: Vec<MeasuredTemperature> = sensors
                    .iter()
                    .map(|sensor| {
                        let meta = sensor.meta.clone();
                        let temperature = sensor.get_temperature();
                        let resolution = sensor.get_resolution();
                        MeasuredTemperature {
                            meta,
                            temperature,
                            resolution,
                        }
                    })
                    .collect();
                // Filter sensors that have any temperature reading
                let sensors: Vec<MeasuredTemperature> = sensors
                    .into_iter()
                    .filter(|sensor| sensor.temperature.is_some())
                    .collect();
                sensors
            }
            // Return empty vector if 1-Wire is disabled
            false => Vec::new(),
        };
        let upses: Vec<UninterruptiblePowerSupplyData> = match enable_ups_monitoring {
            true => {
                // Get data from all UPSes
                upses
                    .iter()
                    .filter_map(|ups| {
                        // Take ownership of client
                        let mut client = match nut_clients.remove(&ups.get_server_id()) {
                            Some(client) => client,
                            None => return None,
                        };
                        // Take ownership of connection
                        let connection = match client.take_connection() {
                            Some(connection) => connection,
                            None => return None,
                        };
                        // Extract variables from UPS
                        let (connection, variables) = ups.list_variables(connection);
                        // Give connection back to client
                        client.give_connection(connection);
                        // Give client back to HashMap
                        nut_clients.insert(client.get_server_id(), client);
                        Some(UninterruptiblePowerSupplyData::new(ups, variables))
                    })
                    .collect()
            }
            // Return empty vector if UPS monitoring is disabled
            false => Vec::new(),
        };
        // Merge data to send into one Object
        let data_to_send = DataToSend::new(sensors, upses);
        // Serialize and log data_to_send
        let serialized_data_to_send =
            serde_json::to_string_pretty(&data_to_send).unwrap_or_default();
        log::debug!("{}", serialized_data_to_send);
        // Send data to all endpoints
        for endpoint in &config.endpoints {
            // Send data to endpoint
            send::send_data(
                &reqwest_client,
                &data_to_send,
                endpoint,
                send_interval,
                ignore_connection_errors,
            );
        }
        // Get end time
        let end_time = Instant::now();
        // Calculate duration
        let used_time = end_time.duration_since(start_time);
        // Calculate time to sleep
        let duration_left_to_sleep = send_interval.checked_sub(used_time).unwrap_or_default();
        // Sleep for sleep_time or at least 500ms
        thread::sleep(cmp::max(duration_left_to_sleep, Duration::from_millis(500)));
    }
}
