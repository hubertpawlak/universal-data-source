// Licensed under the Open Software License version 3.0
use active_sender::receiver::start_active_sender_loop;
use config::file::read_config_or_create_default;
use nut::sender::{start_nut_monitoring_loop, UninterruptiblePowerSupplyData};
use one_wire::sender::{start_one_wire_updater_loop, MeasuredTemperature};
use passive_endpoint::receiver::start_passive_endpoint_loop;
use shutdown_notifier::start_shutdown_notifier;
use tokio::sync::broadcast;
use tracing_subscriber::EnvFilter;
mod active_sender;
mod config;
mod hardware;
mod nut;
mod one_wire;
mod passive_endpoint;
mod shutdown_notifier;

#[tokio::main]
async fn main() {
    // Initialize logger
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive("universal_data_source=info".parse().unwrap())
                .from_env_lossy(),
        )
        .init();

    // Read config file
    let config = read_config_or_create_default();

    // Prepare channels for async tasks
    let (shutdown_tx, shutdown_rx) = broadcast::channel::<()>(1);
    const BROADCAST_CAPACITY: usize = 16;
    let (one_wire_tx, one_wire_rx) =
        broadcast::channel::<Vec<MeasuredTemperature>>(BROADCAST_CAPACITY);
    let (ups_monitoring_tx, ups_monitoring_rx) =
        broadcast::channel::<Vec<UninterruptiblePowerSupplyData>>(BROADCAST_CAPACITY);

    // Gracefully shut down tasks
    // Active sender and passive endpoint shutdown when senders are dropped
    let shutdown_notifier_handle = tokio::spawn(async move {
        start_shutdown_notifier(shutdown_tx).await;
    });

    // Channel receivers
    // Periodically send data to an HTTP endpoint
    let shutdown_rx_clone = shutdown_rx.resubscribe();
    let one_wire_rx_clone = one_wire_rx.resubscribe();
    let ups_monitoring_rx_clone = ups_monitoring_rx.resubscribe();
    let active_sender_handle = tokio::spawn(async move {
        start_active_sender_loop(
            shutdown_rx_clone,
            config.active_data_sender,
            one_wire_rx_clone,
            ups_monitoring_rx_clone,
        )
        .await;
    });

    // Passive endpoint that returns cached data on request
    // Don't clone receivers as this is the last receiving module
    let shutdown_rx_clone = shutdown_rx.resubscribe();
    let passive_endpoint_handle = tokio::spawn(async move {
        start_passive_endpoint_loop(
            shutdown_rx_clone,
            config.passive_data_endpoint,
            one_wire_rx,
            ups_monitoring_rx,
        )
        .await;
    });

    // Channel senders
    // 1-Wire
    let shutdown_rx_clone = shutdown_rx.resubscribe();
    let one_wire_handle = tokio::spawn(async move {
        start_one_wire_updater_loop(shutdown_rx_clone, config.one_wire, one_wire_tx).await
    });

    // Network UPS tools
    // Don't clone shutdown_rx as this is the last module
    let ups_monitoring_handle = tokio::spawn(async move {
        start_nut_monitoring_loop(shutdown_rx, config.ups_monitoring, ups_monitoring_tx).await
    });

    // Join handles
    let _ = tokio::try_join!(
        shutdown_notifier_handle,
        active_sender_handle,
        passive_endpoint_handle,
        one_wire_handle,
        ups_monitoring_handle
    );

    tracing::debug!("Successfully shut down");
}
