// Licensed under the Open Software License version 3.0
use crate::config::types::Example;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Endpoint {
    pub url: String,
    pub bearer_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveSenderConfig {
    enabled: Option<bool>,
    cooldown: Option<Duration>,
    ignore_connection_errors: Option<bool>,
    endpoints: Option<Vec<Endpoint>>,
}

impl Default for ActiveSenderConfig {
    fn default() -> Self {
        Self {
            enabled: Some(false),
            cooldown: Some(Duration::from_secs(10)),
            ignore_connection_errors: Some(false),
            endpoints: None,
        }
    }
}

impl Example for ActiveSenderConfig {
    fn example() -> Self {
        Self {
            enabled: Some(true),
            cooldown: Some(Duration::from_secs(10)),
            ignore_connection_errors: Some(true),
            endpoints: Some(vec![
                Endpoint {
                    url: String::from("http://localhost:3001/anything/status/200"),
                    bearer_token: None,
                },
                Endpoint {
                    url: String::from("https://home-panel.lan/api/trpc/m2m.storeUniversalData"),
                    bearer_token: Some(String::from("EXAMPLE_TOKEN")),
                },
            ]),
        }
    }
}

impl ActiveSenderConfig {
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or_default()
    }

    pub fn get_cooldown(&self) -> Duration {
        self.cooldown.unwrap_or_default()
    }

    pub fn get_endpoints(&self) -> Vec<Endpoint> {
        self.endpoints.clone().unwrap_or_default()
    }

    pub fn get_ignore_connection_errors(&self) -> bool {
        self.ignore_connection_errors.unwrap_or_default()
    }
}
