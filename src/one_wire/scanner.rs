// Licensed under the Open Software License version 3.0
use super::ds18b20::Ds18b20TemperatureSensor;
use std::path::PathBuf;
use tokio::fs::read_dir;

pub async fn get_all_ds18b20_sensors(base_path: &PathBuf) -> Vec<Ds18b20TemperatureSensor> {
    let mut list: Vec<Ds18b20TemperatureSensor> = Vec::new();
    // Return empty list if base_path is not a directory
    if !base_path.is_dir() {
        tracing::error!("base_path is not a directory");
        return list;
    }
    // Read base_path directory
    tracing::trace!("Scanning directory: {}", base_path.display());
    let mut entries = match read_dir(base_path).await {
        Ok(entries) => entries,
        Err(_) => return list,
    };
    // Leave only directories from entries
    // Push instances of valid Ds18b20TemperatureSensors to list
    tracing::trace!("Pushing Ds18b20TemperatureSensors");
    while let Some(entry) = entries.next_entry().await.unwrap() {
        let path = entry.path();
        if path.is_dir() {
            let sensor = Ds18b20TemperatureSensor::new(path.clone());
            // Push if sensor is valid
            if sensor.is_valid() {
                list.push(sensor);
            }
        }
    }
    // Return list
    list
}

#[cfg(test)]
// Use tempfile::TempDir
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_all_ds18b20_sensors() {
        // Create a valid 1-Wire device directory if needed
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path().to_path_buf();
        let device_dir = temp_path.join("28-00000a0b0c0d");
        let temperature_path = device_dir.join("temperature");
        let resolution_path = device_dir.join("resolution");
        std::fs::create_dir(device_dir).unwrap();
        std::fs::write(temperature_path, "1234").unwrap();
        std::fs::write(resolution_path, "12").unwrap();
        // Test get_all_ds18b20_sensors
        let list = get_all_ds18b20_sensors(&temp_path).await;
        assert_eq!(list.len(), 1);
        let sensor = &list[0];
        assert_eq!(sensor.meta.hw.id, "28-00000a0b0c0d");
        assert_eq!(sensor.get_temperature(), Some(1.234));
        assert_eq!(sensor.get_resolution(), Some(12));
    }

    #[tokio::test]
    async fn test_get_all_ds18b20_sensors_empty() {
        // Create a valid 1-Wire device directory if needed
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path().to_path_buf();
        // Test get_all_ds18b20_sensors
        let list = get_all_ds18b20_sensors(&temp_path).await;
        assert_eq!(list.len(), 0);
    }
}
