use bytes::Bytes;
use persistence::WalRecord;
use std::sync::Arc;
use store::{Entry, Store};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

pub async fn run_replica(leader_addr: &str, store: Arc<Store>) -> anyhow::Result<()> {
    tracing::info!(%leader_addr, "connecting to leader for replication");
    let mut stream = TcpStream::connect(leader_addr).await?;

    // 1. Read snapshot.
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let snap_len = u32::from_be_bytes(len_buf) as usize;
    let mut snap_bytes = vec![0u8; snap_len];
    stream.read_exact(&mut snap_bytes).await?;
    let snap: std::collections::HashMap<Vec<u8>, Entry> = bincode::deserialize(&snap_bytes)?;
    let bytes_map: std::collections::HashMap<Bytes, Entry> =
        snap.into_iter().map(|(k, v)| (Bytes::from(k), v)).collect();
    let keys = bytes_map.len();
    store.load(bytes_map).await;
    tracing::info!(keys, "replica loaded snapshot");

    // 2. Loop on records.
    loop {
        let mut len_buf = [0u8; 4];
        if stream.read_exact(&mut len_buf).await.is_err() {
            tracing::warn!("leader connection closed");
            return Ok(());
        }
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut payload = vec![0u8; len];
        stream.read_exact(&mut payload).await?;
        let rec: WalRecord = bincode::deserialize(&payload)?;
        let cmd = rec.to_command();
        store.apply(&cmd).await;
    }
}
