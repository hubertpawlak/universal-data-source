// Licensed under the Open Software License version 3.0
use super::config::{ActiveSenderConfig, Endpoint};
use crate::{nut::sender::UninterruptiblePowerSupplyData, one_wire::sender::MeasuredTemperature};
use serde::{Deserialize, Serialize};
use std::{cmp::max, time::Duration};
use tokio::{
    sync::{broadcast, watch},
    time::Instant,
};
use tokio_stream::StreamExt;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct DataToSend {
    // Has to remain "sensors" for compatibility with home-panel
    sensors: Vec<MeasuredTemperature>,
    upses: Vec<UninterruptiblePowerSupplyData>,
}

impl DataToSend {
    pub fn new(
        sensors: Vec<MeasuredTemperature>,
        upses: Vec<UninterruptiblePowerSupplyData>,
    ) -> Self {
        Self { sensors, upses }
    }
}

pub async fn send_data<T>(
    client: &reqwest::Client,
    json: &T,
    endpoint: &Endpoint,
    timeout: &Duration,
    ignore_connection_errors: &bool,
) where
    T: ?Sized + Serialize,
{
    // Enter send_data span
    // Send json to endpoint
    // With bearer token if available (use empty string if not)
    let result = client
        .post(&endpoint.url)
        .bearer_auth(endpoint.bearer_token.as_deref().unwrap_or(""))
        .json(json)
        .timeout(*timeout)
        .send()
        .await;
    match result {
        Ok(response) => {
            if response.status().is_success() {
                // Pretty-print response object but only in debug mode
                // Used with httpbin to test the request
                #[cfg(debug_assertions)]
                {
                    let json: serde_json::Value = response.json().await.unwrap();
                    tracing::trace!(?json, ?endpoint.url);
                }
            } else {
                // Print response error with endpoint url
                tracing::warn!("Got {} response from {}", response.status(), endpoint.url);
            }
        }
        Err(error) => {
            // Ignore connection errors if specified
            if *ignore_connection_errors && error.is_connect() {
                return;
            }
            tracing::warn!("Connection failed: {}", error);
        }
    }
}

async fn start_active_sender_client_loop(
    mut shutdown_rx: broadcast::Receiver<()>,
    config: ActiveSenderConfig,
    endpoint: Endpoint,
    mut data_to_send_rx: watch::Receiver<DataToSend>,
) {
    // Create a persistent reqwest client
    let client = reqwest::Client::new();
    let cooldown = max(config.get_cooldown(), Duration::from_secs(1));
    // Create in instant at 0 to start sending immediately
    let mut last_sent: Option<Instant> = None;

    loop {
        tokio::select! {
            data_to_send_changed = data_to_send_rx.changed() => {
                if data_to_send_changed.is_err() {
                    tracing::trace!("Shutting down active sender loop for {}", endpoint.url);
                    break;
                }
                if last_sent.is_some() && last_sent.unwrap().elapsed() <= cooldown {
                    tracing::trace!("Skipping because of cooldown: {}", endpoint.url);
                    continue;
                }
                let data_to_send = data_to_send_rx.borrow().clone();
                send_data(
                    &client,
                    &data_to_send,
                    &endpoint,
                    &Duration::from_secs(5),
                    &config.get_ignore_connection_errors(),
                )
                .await;
                last_sent = Some(Instant::now());
            }
            _ = shutdown_rx.recv() => {
                tracing::trace!("Shutting down active sender loop for {}", endpoint.url);
                break;
            }
        }
    }
}

pub async fn start_active_sender_loop(
    mut shutdown_rx: broadcast::Receiver<()>,
    config: ActiveSenderConfig,
    mut one_wire_rx: broadcast::Receiver<Vec<MeasuredTemperature>>,
    mut ups_monitoring_rx: broadcast::Receiver<Vec<UninterruptiblePowerSupplyData>>,
) {
    // Check if module is enabled
    if !config.is_enabled() {
        tracing::trace!("Module is disabled");
        return;
    }

    // Prepare channel with merged data
    let (data_to_send_tx, data_to_send_rx) = watch::channel::<DataToSend>(DataToSend {
        sensors: vec![],
        upses: vec![],
    });

    // Spawn task for each endpoint
    tracing::trace!("Starting active sender loop");
    let endpoints = config.get_endpoints();
    let mut endpoints = tokio_stream::iter(endpoints);

    // Make sure all tasks are spawned
    let mut tasks = Vec::new();

    while let Some(endpoint) = endpoints.next().await {
        let shutdown_rx_clone = shutdown_rx.resubscribe();
        let data_to_send_rx = data_to_send_rx.clone();
        let config = config.clone();
        let task = tokio::spawn(async move {
            start_active_sender_client_loop(shutdown_rx_clone, config, endpoint, data_to_send_rx)
                .await
        });
        tasks.push(task);
    }

    let data_merger_task = tokio::spawn(async move {
        let mut data_to_send = DataToSend::new(vec![], vec![]);
        loop {
            tokio::select! {
                Ok(value) = one_wire_rx.recv() => {
                    tracing::trace!("one_wire_changed");
                    data_to_send.sensors = value;
                    data_to_send_tx.send(data_to_send.clone()).unwrap();
                }
                Ok(value) = ups_monitoring_rx.recv() => {
                    tracing::trace!("ups_monitoring_received");
                    data_to_send.upses = value;
                    data_to_send_tx.send(data_to_send.clone()).unwrap();
                }
                _ = shutdown_rx.recv() => {
                    tracing::trace!("Shutting down data merger task");
                    break;
                }
            }
        }
    });
    tasks.push(data_merger_task);

    // Await all tasks
    for task in tasks {
        task.await.unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Matcher::JsonString, Server};
    use reqwest::Client;
    use std::time::Duration;

    #[tokio::test]
    async fn test_send_data_without_token() {
        // Mock server
        let mut server = Server::new();
        // Prepare fake endpoint
        let mock = server
            .mock("POST", "/post-data")
            .match_body(JsonString(r#"[1, 2, 3, 4, 5]"#.to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"json": [1, 2, 3, 4, 5]}"#)
            .create();
        // Send data
        let client = Client::new();
        let endpoint = Endpoint {
            url: format!("{}{}", server.url(), "/post-data"),
            bearer_token: None,
        };
        let timeout = Duration::from_secs(5);
        let data = vec![1, 2, 3, 4, 5];
        send_data(&client, &data, &endpoint, &timeout, &false).await;
        // Assert that mock was called
        mock.assert();
    }

    #[tokio::test]
    async fn test_send_data_with_bearer_token() {
        let mut server = Server::new();
        let mock = server
            .mock("POST", "/post-data")
            .match_body(JsonString(r#"[1, 2, 3, 4, 5]"#.to_string()))
            .match_header("Authorization", "Bearer token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"json": [1, 2, 3, 4, 5]}"#)
            .create();
        let bearer_token = Some("token".to_string());
        let client = Client::new();
        let endpoint = Endpoint {
            url: format!("{}{}", server.url(), "/post-data"),
            bearer_token,
        };
        let timeout = Duration::from_secs(5);
        let data = vec![1, 2, 3, 4, 5];
        send_data(&client, &data, &endpoint, &timeout, &false).await;
        mock.assert();
    }
}
