// Licensed under the Open Software License version 3.0
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceType {
    OneWire,
    NetworkUpsTools,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HardwareType {
    TemperatureSensor,
    UninterruptiblePowerSupply,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub id: String,
    pub hardware_type: HardwareType,
}

impl HardwareInfo {
    pub fn new(id: String, hardware_type: HardwareType) -> Self {
        Self { id, hardware_type }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceInfo {
    pub source_type: SourceType,
}

impl SourceInfo {
    pub fn new(source_type: SourceType) -> Self {
        Self { source_type }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareMetadata {
    pub hw: HardwareInfo,
    pub source: SourceInfo,
}

impl HardwareMetadata {
    pub fn new(id: String, hardware_type: HardwareType, source_type: SourceType) -> Self {
        Self {
            hw: HardwareInfo::new(id, hardware_type),
            source: SourceInfo::new(source_type),
        }
    }
}
