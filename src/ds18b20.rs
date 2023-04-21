// Licensed under the Open Software License version 3.0
use crate::hardware::{HardwareMetadata, HardwareType, SourceType};
use regex::Regex;
use serde::{Serialize, Serializer};
use std::{fs::read_to_string, path::PathBuf};

/// `Ds18b20TemperatureSensor`
/// represents a 1-Wire temperature sensor (ex. DS18B20).
/// It needs to have both `temperature`
/// and `resolution` files inside its directory
pub struct Ds18b20TemperatureSensor {
    pub meta: HardwareMetadata,
    path: PathBuf,
}

// Convert path to string for serialization
impl Serialize for Ds18b20TemperatureSensor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let path = self.path.to_str().unwrap();
        serializer.serialize_str(path)
    }
}

#[derive(Serialize)]
pub struct MeasuredTemperature {
    pub meta: HardwareMetadata,
    pub temperature: Option<f64>,
    pub resolution: Option<u8>,
}

const ONE_WIRE_DEVICE_ID_REGEX: &str = r"^[0-9a-f]{2}-[0-9a-f]{12}$";

impl Ds18b20TemperatureSensor {
    // Create new instance from path
    pub fn new(path: PathBuf) -> Self {
        // Take id from path's dir name
        let id = path.file_name().unwrap().to_str().unwrap().to_string();
        // Create
        Self {
            meta: HardwareMetadata::new(id, HardwareType::TemperatureSensor, SourceType::OneWire),
            path,
        }
    }
    pub fn is_valid(&self) -> bool {
        // Path must be a directory
        if !self.path.is_dir() {
            return false;
        }
        // Path must match 1-Wire device id regex
        let id = self.path.file_name().unwrap().to_str().unwrap();
        if !(Regex::new(ONE_WIRE_DEVICE_ID_REGEX).unwrap().is_match(id)) {
            return false;
        }
        // Path must contain both "temperature" and "resolution" files that exist
        let temperature_path = self.path.join("temperature");
        if !temperature_path.is_file() {
            return false;
        }
        let resolution_path = self.path.join("resolution");
        if !resolution_path.is_file() {
            return false;
        }
        true
    }
    // Optionally get temperature from file
    pub fn get_temperature(&self) -> Option<f64> {
        // Check if "temperature" file inside path exists
        // Return an error if it doesn't but don't panic
        let path = self.path.join("temperature");
        if !path.is_file() {
            return None;
        }
        // Read file
        let contents = read_to_string(path).unwrap();
        // Try to parse file contents as f64, handle error and convert from millicelsius to celsius
        let temperature = match contents.trim().parse::<f64>() {
            Ok(temperature) => temperature / 1000.0,
            Err(_) => return None,
        };
        // Return temperature
        Some(temperature)
    }
    pub fn get_resolution(&self) -> Option<u8> {
        // Check if "resolution" file inside path exists
        // Return an error if it doesn't but don't panic
        let path = self.path.join("resolution");
        if !path.is_file() {
            return None;
        }
        // Read file
        let contents = read_to_string(path).unwrap();
        // Try to parse file contents as u8, handle error
        let resolution = match contents.trim().parse::<u8>() {
            Ok(resolution) => resolution,
            Err(_) => return None,
        };
        // Return resolution
        Some(resolution)
    }
}

#[cfg(test)]
// Use tempfile::TempDir
// Create a valid 1-Wire device directory if needed
// Don't repeat yourself
mod tests {
    use super::*;
    use tempfile::{tempdir, TempDir};

    const VALID_DEVICE_ID: &str = "28-00000a0b0c0d";

    fn create_valid_device_dir() -> TempDir {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        let device_dir = temp_path.join(VALID_DEVICE_ID);
        let temperature_path = device_dir.join("temperature");
        let resolution_path = device_dir.join("resolution");
        std::fs::create_dir(device_dir).unwrap();
        std::fs::write(temperature_path, "1234").unwrap();
        std::fs::write(resolution_path, "12").unwrap();
        temp_dir
    }

    #[test]
    fn new() {
        let temp_dir = create_valid_device_dir();
        let temp_path = temp_dir.path();
        let device_dir = temp_path.join(VALID_DEVICE_ID);
        let sensor = Ds18b20TemperatureSensor::new(device_dir);
        assert_eq!(sensor.meta.hw.id, VALID_DEVICE_ID);
        assert_eq!(sensor.path, temp_path.join(VALID_DEVICE_ID));
    }

    #[test]
    fn is_valid() {
        // Create a valid device dir
        let temp_dir = create_valid_device_dir();
        let temp_path = temp_dir.path();
        let device_dir = temp_path.join(VALID_DEVICE_ID);
        // Create new sensor from device dir
        let sensor = Ds18b20TemperatureSensor::new(device_dir);
        // Check if sensor is valid
        assert!(sensor.is_valid());
    }

    #[test]
    fn is_valid_invalid_dir() {
        // Create a temp dir
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        let device_dir = temp_path.join(VALID_DEVICE_ID);
        // Create new sensor from device dir
        let sensor = Ds18b20TemperatureSensor::new(device_dir);
        // Check if sensor is valid
        assert!(!sensor.is_valid());
    }

    #[test]
    fn is_valid_invalid_id() {
        // Create a temp dir
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        // Too short device id
        let device_dir = temp_path.join("28-0123456789a");
        // Create new sensor from device dir
        let sensor = Ds18b20TemperatureSensor::new(device_dir);
        // Check if sensor is valid
        assert!(!sensor.is_valid());
    }

    #[test]
    fn get_temperature() {
        // Create a valid device dir
        let temp_dir = create_valid_device_dir();
        let temp_path = temp_dir.path();
        let device_dir = temp_path.join(VALID_DEVICE_ID);
        // Create new sensor from device dir
        let sensor = Ds18b20TemperatureSensor::new(device_dir);
        // Get temperature
        let temperature = sensor.get_temperature();
        // Check if temperature is valid
        assert!(temperature.is_some());
        // Check if temperature is 1.234
        assert_eq!(temperature.unwrap(), 1.234);
    }

    #[test]
    fn get_temperature_invalid() {
        // Create a temp dir
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        let device_dir = temp_path.join(VALID_DEVICE_ID);
        // Create new sensor from device dir
        let sensor = Ds18b20TemperatureSensor::new(device_dir);
        // Get temperature
        let temperature = sensor.get_temperature();
        // Check if temperature is valid
        assert!(temperature.is_none());
    }

    #[test]
    fn get_resolution() {
        // Create a valid device dir
        let temp_dir = create_valid_device_dir();
        let temp_path = temp_dir.path();
        let device_dir = temp_path.join(VALID_DEVICE_ID);
        // Create new sensor from device dir
        let sensor = Ds18b20TemperatureSensor::new(device_dir);
        // Get resolution
        let resolution = sensor.get_resolution();
        // Check if resolution is valid
        assert!(resolution.is_some());
        // Check if resolution is 12
        assert_eq!(resolution.unwrap(), 12);
    }

    #[test]
    fn get_resolution_invalid() {
        // Create a temp dir
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();
        let device_dir = temp_path.join(VALID_DEVICE_ID);
        // Create new sensor from device dir
        let sensor = Ds18b20TemperatureSensor::new(device_dir);
        // Get resolution
        let resolution = sensor.get_resolution();
        // Check if resolution is valid
        assert!(resolution.is_none());
    }
}
