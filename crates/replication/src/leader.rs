use bincode;
use command::Command;
use persistence::WalRecord;
use std::sync::Arc;
use store::Store;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;

/// The leader broadcasts WalRecord events to replicas via this channel.
pub type ReplStream = broadcast::Sender<WalRecord>;

pub fn channel() -> ReplStream {
    broadcast::channel(1024).0
}

/// Notify replicas of a committed mutating command. No-op for read-only.
pub fn broadcast_command(tx: &ReplStream, cmd: &Command) {
    if let Some(rec) = WalRecord::from_command(cmd) {
        let _ = tx.send(rec);
    }
}

/// Run the replication listener on a separate port for replicas to connect.
pub async fn run_replication_listener(
    addr: &str,
    tx: ReplStream,
    store: Arc<Store>,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    tracing::info!(%addr, "replication listener up");
    loop {
        let (stream, peer) = listener.accept().await?;
        tracing::info!(?peer, "replica connected");
        let rx = tx.subscribe();
        let store = store.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_replica(stream, rx, store).await {
                tracing::warn!(error = %e, "replica conn failed");
            }
        });
    }
}

async fn handle_replica(
    mut stream: TcpStream,
    mut rx: broadcast::Receiver<WalRecord>,
    store: Arc<Store>,
) -> anyhow::Result<()> {
    // 1. Send snapshot.
    let snapshot = store.snapshot().await;
    let serializable: std::collections::HashMap<Vec<u8>, store::Entry> = snapshot
        .iter()
        .map(|(k, v)| (k.to_vec(), v.clone()))
        .collect();
    let snap_bytes = bincode::serialize(&serializable)?;
    stream
        .write_all(&(snap_bytes.len() as u32).to_be_bytes())
        .await?;
    stream.write_all(&snap_bytes).await?;
    stream.flush().await?;
    tracing::info!(bytes = snap_bytes.len(), "snapshot sent to replica");

    // 2. Stream subsequent commands.
    loop {
        let rec = rx.recv().await?;
        let bytes = bincode::serialize(&rec)?;
        stream
            .write_all(&(bytes.len() as u32).to_be_bytes())
            .await?;
        stream.write_all(&bytes).await?;
        stream.flush().await?;
    }
}
