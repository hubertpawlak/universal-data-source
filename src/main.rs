// Licensed under the Open Software License version 3.0
use crate::config::create_default_config_if_not_exists;
use ds18b20::MeasuredTemperature;
use std::{
    cmp,
    path::PathBuf,
    process, thread,
    time::{Duration, Instant},
};
mod config;
mod ds18b20;
mod hardware;
mod scanner;
mod send;

fn main() {
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
            println!("Error: {}", error);
            // Write default config to file
            if create_default_config_if_not_exists(&config_file_path) {
                println!("Wrote default config to config.json");
                println!("Please edit config.json and try again");
            } else {
                println!("Failed to create default config.json");
                println!("Try manually deleting config.json and running again");
            }
            process::exit(1);
        }
    };

    let one_wire_path_prefix = PathBuf::from(&config.one_wire_path_prefix.unwrap_or_default());
    // Loop every send_interval seconds
    // Send data to all endpoints
    // Send data from all sensors
    loop {
        // Get start time
        let start_time = Instant::now();
        // Find all sensors
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
        // Send data to all endpoints
        for endpoint in &config.endpoints {
            // Send data to endpoint
            send::send_data(
                &sensors,
                endpoint,
                &config.send_interval.unwrap_or_default(),
            );
        }
        // Get end time
        let end_time = Instant::now();
        // Calculate duration
        let used_time = end_time.duration_since(start_time);
        // Calculate time to sleep
        let duration_left_to_sleep = config.send_interval.unwrap_or_default() - used_time;
        // Sleep for sleep_time or at least 500ms
        thread::sleep(cmp::max(duration_left_to_sleep, Duration::from_millis(500)));
    }
}
