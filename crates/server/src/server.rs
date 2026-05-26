use crate::conn::handle_connection;
use persistence::{Snapshot, Wal};
use replication::{channel, run_replica, run_replication_listener, ReplStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use store::Store;
use tokio::net::TcpListener;

pub struct Server {
    pub port: u16,
    pub repl_port: u16,
    pub data_dir: PathBuf,
    pub fsync_every: bool,
    pub replicaof: Option<String>,
}

impl Server {
    pub async fn run(self) -> anyhow::Result<()> {
        tokio::fs::create_dir_all(&self.data_dir).await?;
        let snap = Snapshot::new(self.data_dir.join("snapshot.rdb"));
        let wal = Arc::new(Wal::open(self.data_dir.join("wal.log"), self.fsync_every).await?);

        let store = Store::new();
        if let Some(d) = snap.read().await? {
            store.load(d).await;
        }
        for r in wal.replay().await? {
            store.apply(&r.to_command()).await;
        }

        let read_only = self.replicaof.is_some();

        // If we're a replica, kick off the follower task.
        if let Some(leader) = &self.replicaof {
            let s = store.clone();
            let leader_addr = leader.clone();
            tokio::spawn(async move {
                if let Err(e) = run_replica(&leader_addr, s).await {
                    tracing::error!(error = %e, "replica task exited");
                }
            });
        }

        // If we're a leader, kick off the replication listener.
        let repl_tx: Option<ReplStream> = if !read_only {
            let tx = channel();
            let s = store.clone();
            let addr = format!("0.0.0.0:{}", self.repl_port);
            let tx_for_task = tx.clone();
            tokio::spawn(async move {
                if let Err(e) = run_replication_listener(&addr, tx_for_task, s).await {
                    tracing::error!(error = %e, "replication listener exited");
                }
            });
            Some(tx)
        } else {
            None
        };

        // TTL sweep.
        let s = store.clone();
        tokio::spawn(async move {
            let mut t = tokio::time::interval(Duration::from_secs(5));
            loop {
                t.tick().await;
                s.sweep_expired().await;
            }
        });

        // Periodic snapshot (leader only).
        if !read_only {
            let snap_store = store.clone();
            let snap_path = self.data_dir.join("snapshot.rdb");
            let wal2 = wal.clone();
            tokio::spawn(async move {
                let mut t = tokio::time::interval(Duration::from_secs(300));
                t.tick().await;
                let snap = Snapshot::new(&snap_path);
                loop {
                    t.tick().await;
                    let data = snap_store.snapshot().await;
                    if snap.write(&data).await.is_ok() {
                        let _ = wal2.truncate().await;
                    }
                }
            });
        }

        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        tracing::info!(%addr, role = if read_only { "replica" } else { "leader" }, "listening");

        loop {
            let (s, _) = listener.accept().await?;
            let store = store.clone();
            let wal = wal.clone();
            let tx = repl_tx.clone();
            tokio::spawn(async move {
                let _ = handle_connection(s, store, wal, tx, read_only).await;
            });
        }
    }
}
