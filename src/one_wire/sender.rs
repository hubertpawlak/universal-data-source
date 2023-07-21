// Licensed under the Open Software License version 3.0
use super::{config::OneWireConfig, scanner::get_all_ds18b20_sensors};
use crate::{
    config::types::Example,
    hardware::types::{HardwareMetadata, HardwareType, SourceType},
};
use serde::{Deserialize, Serialize};
use std::{cmp::max, time::Duration};
use tokio::{sync::broadcast, time::sleep};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeasuredTemperature {
    pub meta: HardwareMetadata,
    pub temperature: Option<f64>,
    pub resolution: Option<u8>,
}

impl Example for MeasuredTemperature {
    /// Create an instance of `MeasuredTemperature` for internal testing
    ///
    /// Default `temperature` is 0
    ///
    /// Default `resolution` is 12
    fn example() -> Self {
        Self {
            meta: HardwareMetadata::new(
                String::from("fake_hw_id"),
                HardwareType::TemperatureSensor,
                SourceType::OneWire,
            ),
            temperature: Some(0.0),
            resolution: Some(12),
        }
    }
}

pub async fn start_one_wire_updater_loop(
    mut shutdown_rx: broadcast::Receiver<()>,
    config: OneWireConfig,
    tx: broadcast::Sender<Vec<MeasuredTemperature>>,
) {
    // Check if module is enabled
    if !config.is_enabled() {
        tracing::trace!("Module is disabled");
        return;
    }
    tracing::debug!("Starting one wire updater loop");
    // Extract config fields
    let base_path = config.get_base_path();
    let cooldown = max(config.get_cooldown(), Duration::from_millis(200));
    // Start measuring temperature
    loop {
        // Find all sensors - calling inside loop makes sensors hot-swappable
        let sensors = get_all_ds18b20_sensors(&base_path).await;
        // Map additional fields: temperature and resolution
        tracing::trace!("Mapping temperature and resolution");
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
        tracing::trace!("Filtering empty readings");
        let sensors: Vec<MeasuredTemperature> = sensors
            .into_iter()
            .filter(|sensor| sensor.temperature.is_some())
            .collect();
        tracing::trace!("Sending {:?} to channel", sensors);
        if tx.receiver_count() > 0 {
            tx.send(sensors).unwrap();
        }
        tokio::select! {
            _ = shutdown_rx.recv() => {
                tracing::trace!("Shutting down one wire updater loop");
                break;
            }
            _ = sleep(cooldown) => {}
        }
    }
}
