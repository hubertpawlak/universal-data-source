// Licensed under the Open Software License version 3.0
use crate::hardware::{HardwareMetadata, HardwareType, SourceType};
use rups::blocking::Connection;
use serde::Serialize;
use std::collections::HashMap;

pub struct UninterruptiblePowerSupply {
    pub meta: HardwareMetadata,
    server_id: String,
    ups_name: String,
    variables_to_monitor: Vec<String>,
}

const DEFAULT_VARIABLES_TO_MONITOR: [&str; 15] = [
    "battery.charge",
    "battery.charge.low",
    "battery.runtime",
    "battery.runtime.low",
    "input.frequency",
    "input.voltage",
    "output.frequency",
    "output.frequency.nominal",
    "output.voltage",
    "output.voltage.nominal",
    "ups.load",
    "ups.power",
    "ups.power.nominal",
    "ups.realpower",
    "ups.status",
];

impl UninterruptiblePowerSupply {
    pub fn new(
        server_id: String,
        ups_name: String,
        variables_to_monitor: Option<Vec<String>>,
    ) -> Self {
        // Create id by prepending "[ups_name]" to "server_id"
        let id = format!("[{}]{}", ups_name, server_id);
        Self {
            meta: HardwareMetadata::new(
                id,
                HardwareType::UninterruptiblePowerSupply,
                SourceType::NetworkUpsTools,
            ),
            ups_name,
            server_id,
            variables_to_monitor: variables_to_monitor.unwrap_or(
                DEFAULT_VARIABLES_TO_MONITOR
                    .iter()
                    .map(|v| v.to_string())
                    .collect(),
            ),
        }
    }

    pub fn get_server_id(&self) -> String {
        self.server_id.clone()
    }

    // Always return borrowed Connection
    pub fn list_variables(
        &self,
        mut connection: Connection,
    ) -> (Connection, HashMap<String, String>) {
        // Get values for variables inside variables_to_get
        let variables: HashMap<String, String> = HashMap::from_iter(
            self.variables_to_monitor
                .iter()
                .filter_map(|variable_to_get| {
                    match connection.get_var(&self.ups_name, variable_to_get) {
                        Ok(returned_variable) => Some((
                            returned_variable.name().to_string(),
                            returned_variable.value(),
                        )),
                        Err(_) => None,
                    }
                }),
        );
        // Return connection ownership and variables as key-value hashmap
        (connection, variables)
    }
}

#[derive(Serialize)]
pub struct UninterruptiblePowerSupplyData {
    pub meta: HardwareMetadata,
    pub variables: HashMap<String, String>,
}

impl UninterruptiblePowerSupplyData {
    pub fn new(ups: &UninterruptiblePowerSupply, variables: HashMap<String, String>) -> Self {
        Self {
            meta: ups.meta.clone(),
            variables,
        }
    }
}
