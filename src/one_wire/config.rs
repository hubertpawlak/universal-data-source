// Licensed under the Open Software License version 3.0
use crate::config::types::Example;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Duration};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OneWireConfig {
    enabled: Option<bool>,
    base_path: Option<String>,
    cooldown: Option<Duration>,
}

impl Default for OneWireConfig {
    // Fallback values
    fn default() -> Self {
        Self {
            enabled: Some(false),
            base_path: Some(String::from("/sys/bus/w1/devices")),
            cooldown: Some(Duration::from_secs(1)),
        }
    }
}

impl Example for OneWireConfig {
    fn example() -> Self {
        Self {
            enabled: Some(true),
            base_path: Some(String::from("/sys/bus/w1/devices")),
            cooldown: Some(Duration::from_secs(1)),
        }
    }
}

impl OneWireConfig {
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or_default()
    }

    pub fn get_base_path(&self) -> PathBuf {
        PathBuf::from(self.base_path.clone().unwrap_or_default())
    }

    pub fn get_cooldown(&self) -> Duration {
        self.cooldown.unwrap_or_default()
    }
}
