use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "mini-redis", version, about = "A Redis-compatible KV store in Rust")]
struct Args {
    /// TCP port to bind
    #[arg(short, long, default_value_t = 6380)]
    port: u16,

    /// Data directory for WAL + snapshots
    #[arg(short, long, default_value = "./data")]
    data_dir: PathBuf,

    /// Run as replica of leader at host:port
    #[arg(long)]
    replicaof: Option<String>,

    /// fsync on every WAL append (slower, stronger durability)
    #[arg(long)]
    fsync_every_write: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    tracing::info!(?args, "starting mini-redis");

    // Phase 1 stub — real server boot lands in Task 3.6
    tracing::info!("server scaffold ready (no listener yet)");
    Ok(())
}
