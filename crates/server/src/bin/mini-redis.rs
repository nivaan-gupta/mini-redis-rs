use clap::Parser;
use server::Server;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "mini-redis", version)]
struct Args {
    #[arg(short, long, default_value_t = 6380)]
    port: u16,
    #[arg(long, default_value_t = 6390)]
    repl_port: u16,
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
    Server {
        port: args.port,
        repl_port: args.repl_port,
        data_dir: args.data_dir,
        fsync_every: args.fsync_every_write,
        replicaof: args.replicaof,
    }
    .run()
    .await
}
