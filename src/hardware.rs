// Licensed under the Open Software License version 3.0
use serde::Serialize;

#[derive(Serialize, Clone)]
pub enum SourceType {
    OneWire,
    NetworkUpsTools,
}

#[derive(Serialize, Clone)]
pub enum HardwareType {
    TemperatureSensor,
    UninterruptiblePowerSupply,
}

#[derive(Serialize, Clone)]
pub struct HardwareInfo {
    pub id: String,
    pub hardware_type: HardwareType,
}

impl HardwareInfo {
    pub fn new(id: String, hardware_type: HardwareType) -> Self {
        Self { id, hardware_type }
    }
}

#[derive(Serialize, Clone)]
pub struct SourceInfo {
    pub source_type: SourceType,
}

impl SourceInfo {
    pub fn new(source_type: SourceType) -> Self {
        Self { source_type }
    }
}

#[derive(Serialize, Clone)]
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
