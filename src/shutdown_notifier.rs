// Licensed under the Open Software License version 3.0
use tokio::sync::broadcast::Sender;

pub async fn start_shutdown_notifier(tx: Sender<()>) {
    tracing::trace!("Starting shutdown notifier");
    let _ = tokio::signal::ctrl_c().await;
    tracing::debug!("Received shutdown signal");
    tracing::trace!("Sending message to {} receivers", tx.receiver_count());
    let _ = tx.send(());
}
