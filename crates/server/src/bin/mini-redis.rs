use clap::Parser;
use server::Server;
use std::path::PathBuf;
use store::Store;

#[derive(Parser, Debug)]
#[command(
    name = "mini-redis",
    version,
    about = "A Redis-compatible KV store in Rust"
)]
struct Args {
    #[arg(short, long, default_value_t = 6380)]
    port: u16,
    #[arg(short, long, default_value = "./data")]
    data_dir: PathBuf,
    #[arg(long)]
    replicaof: Option<String>,
    #[arg(long)]
    fsync_every_write: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let args = Args::parse();
    tracing::info!(?args, "starting mini-redis");

    let store = Store::new();
    Server::new(args.port, store).run().await
}
