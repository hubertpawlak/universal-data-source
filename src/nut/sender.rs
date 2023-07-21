// Licensed under the Open Software License version 3.0
use super::{
    client::{NetworkUpsToolsClient, UninterruptiblePowerSupply},
    config::{NetworkUpsToolsClientConfig, UpsMonitoringConfig},
};
use crate::{
    config::types::Example,
    hardware::types::{HardwareMetadata, HardwareType, SourceType},
};
use serde::{Deserialize, Serialize};
use std::{cmp::max, collections::HashMap, time::Duration};
use tokio::{sync::broadcast, time::sleep};
use tokio_stream::StreamExt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UninterruptiblePowerSupplyData {
    pub meta: HardwareMetadata,
    pub variables: HashMap<String, String>,
}

impl Example for UninterruptiblePowerSupplyData {
    /// Create an instance of `UninterruptiblePowerSupplyData` for internal testing
    ///
    /// Default `battery.charge` is 100
    ///
    /// Default `ups.load` is 15
    fn example() -> Self {
        let mut variables = HashMap::new();
        variables.insert(String::from("battery.charge"), String::from("100"));
        variables.insert(String::from("ups.load"), String::from("15"));

        Self {
            meta: HardwareMetadata::new(
                String::from("fake_hw_id"),
                HardwareType::UninterruptiblePowerSupply,
                SourceType::NetworkUpsTools,
            ),
            variables,
        }
    }
}

impl UninterruptiblePowerSupplyData {
    pub fn new(ups: &UninterruptiblePowerSupply, variables: HashMap<String, String>) -> Self {
        Self {
            meta: ups.meta.clone(),
            variables,
        }
    }
}

async fn start_nut_client_loop(
    mut shutdown_rx: broadcast::Receiver<()>,
    server_config: NetworkUpsToolsClientConfig,
    tx: broadcast::Sender<Vec<UninterruptiblePowerSupplyData>>,
    cooldown: Duration,
) {
    tracing::trace!(
        "Starting nut client loop for {}",
        server_config.get_server_id()
    );
    let client = NetworkUpsToolsClient::new(&server_config, cooldown);
    loop {
        let upses_with_variables = client.query_all_upses().await;
        if tx.receiver_count() > 0 {
            tx.send(upses_with_variables).unwrap();
        }
        tokio::select! {
            _ = shutdown_rx.recv() => {
                tracing::trace!("Shutting down nut client loop for {}", server_config.get_server_id());
                break;
            }
            _ = sleep(cooldown) => {}
        }
    }
    tracing::trace!(
        "Stopped nut client loop for {}",
        server_config.get_server_id()
    );
}

pub async fn start_nut_monitoring_loop(
    shutdown_rx: broadcast::Receiver<()>,
    config: UpsMonitoringConfig,
    tx: broadcast::Sender<Vec<UninterruptiblePowerSupplyData>>,
) {
    // Check if module is enabled
    if !config.is_enabled() {
        tracing::trace!("Module is disabled");
        return;
    }

    // Spawn task for each server
    tracing::trace!("Starting nut monitoring loop");
    let cooldown = max(config.get_cooldown(), Duration::from_millis(200));
    let server_configs = config.get_server_configs();
    let mut server_configs = tokio_stream::iter(server_configs);

    while let Some(server_config) = server_configs.next().await {
        let shutdown_rx_clone = shutdown_rx.resubscribe();
        let tx = tx.clone();
        tokio::spawn(async move {
            start_nut_client_loop(shutdown_rx_clone, server_config, tx, cooldown).await;
        })
        .await
        .unwrap()
    }
}
