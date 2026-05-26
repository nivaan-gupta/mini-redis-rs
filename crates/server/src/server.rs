use crate::conn::handle_connection;
use persistence::{Snapshot, Wal};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use store::Store;
use tokio::net::TcpListener;

pub struct Server {
    port: u16,
    data_dir: PathBuf,
    fsync_every: bool,
}

impl Server {
    pub fn new(port: u16, data_dir: PathBuf, fsync_every: bool) -> Self {
        Self {
            port,
            data_dir,
            fsync_every,
        }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        tokio::fs::create_dir_all(&self.data_dir).await?;
        let snap_path = self.data_dir.join("snapshot.rdb");
        let wal_path = self.data_dir.join("wal.log");

        // 1. Load snapshot if present.
        let store = Store::new();
        let snap = Snapshot::new(&snap_path);
        if let Some(data) = snap.read().await? {
            tracing::info!(keys = data.len(), "loaded snapshot");
            store.load(data).await;
        }

        // 2. Replay WAL.
        let wal = Arc::new(Wal::open(&wal_path, self.fsync_every).await?);
        let records = wal.replay().await?;
        if !records.is_empty() {
            tracing::info!(records = records.len(), "replaying WAL");
            for r in &records {
                store.apply(&r.to_command()).await;
            }
        }

        // 3. Start TCP listener.
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        tracing::info!(%addr, "listening");

        // 4. Background TTL sweep.
        let sweep_store = store.clone();
        tokio::spawn(async move {
            let mut t = tokio::time::interval(Duration::from_secs(5));
            loop {
                t.tick().await;
                sweep_store.sweep_expired().await;
            }
        });

        // 5. Periodic snapshot every 5 min.
        let snap_store = store.clone();
        let snap_for_task = Snapshot::new(&snap_path);
        let wal_for_task = wal.clone();
        tokio::spawn(async move {
            let mut t = tokio::time::interval(Duration::from_secs(300));
            t.tick().await; // skip first immediate tick
            loop {
                t.tick().await;
                let data = snap_store.snapshot().await;
                if let Err(e) = snap_for_task.write(&data).await {
                    tracing::warn!(error = %e, "snapshot failed");
                    continue;
                }
                if let Err(e) = wal_for_task.truncate().await {
                    tracing::warn!(error = %e, "wal truncate failed");
                }
                tracing::info!("snapshot + wal truncate complete");
            }
        });

        // 6. Accept loop.
        loop {
            let (stream, peer) = listener.accept().await?;
            tracing::debug!(?peer, "accepted");
            let store = store.clone();
            let wal = wal.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, store, wal).await {
                    tracing::warn!(error = %e, "connection error");
                }
            });
        }
    }
}
