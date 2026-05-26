use crate::conn::handle_connection;
use std::sync::Arc;
use std::time::Duration;
use store::Store;
use tokio::net::TcpListener;

pub struct Server {
    addr: String,
    store: Arc<Store>,
}

impl Server {
    pub fn new(port: u16, store: Arc<Store>) -> Self {
        Self {
            addr: format!("0.0.0.0:{port}"),
            store,
        }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(&self.addr).await?;
        tracing::info!(addr = %self.addr, "listening");

        // Background TTL sweep every 5s.
        let sweep_store = self.store.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(5));
            loop {
                ticker.tick().await;
                sweep_store.sweep_expired().await;
            }
        });

        loop {
            let (stream, peer) = listener.accept().await?;
            tracing::debug!(?peer, "accepted connection");
            let store = self.store.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, store).await {
                    tracing::warn!(error = %e, "connection error");
                }
            });
        }
    }
}
