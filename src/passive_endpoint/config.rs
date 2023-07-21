// Licensed under the Open Software License version 3.0
use crate::config::types::Example;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PassiveEndpointConfig {
    enabled: Option<bool>,
    port: Option<u16>,
}

impl Default for PassiveEndpointConfig {
    fn default() -> Self {
        Self {
            enabled: Some(false),
            port: Some(63623),
        }
    }
}

impl Example for PassiveEndpointConfig {
    fn example() -> Self {
        Self {
            enabled: Some(true),
            port: Some(63623),
        }
    }
}

impl PassiveEndpointConfig {
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or_default()
    }

    pub fn get_port(&self) -> u16 {
        self.port.unwrap_or_default()
    }
}
