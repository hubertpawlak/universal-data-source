// Licensed under the Open Software License version 3.0
use crate::active_sender::config::ActiveSenderConfig;
use crate::nut::config::UpsMonitoringConfig;
use crate::one_wire::config::OneWireConfig;
use crate::passive_endpoint::config::PassiveEndpointConfig;
use serde::{Deserialize, Serialize};

// Values to generate example config file
pub trait Example {
    fn example() -> Self;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
/// `Config` struct for deserializing config.json
pub struct Config {
    pub one_wire: OneWireConfig,
    pub ups_monitoring: UpsMonitoringConfig,
    pub active_data_sender: ActiveSenderConfig,
    pub passive_data_endpoint: PassiveEndpointConfig,
}

impl Example for Config {
    fn example() -> Self {
        Self {
            one_wire: OneWireConfig::example(),
            ups_monitoring: UpsMonitoringConfig::example(),
            active_data_sender: ActiveSenderConfig::example(),
            passive_data_endpoint: PassiveEndpointConfig::example(),
        }
    }
}
