use bytes::BytesMut;
use command::parse_command;
use persistence::{Wal, WalRecord};
use protocol::{encode, parse, RespError, RespValue};
use replication::ReplStream;
use std::sync::Arc;
use store::Store;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub async fn handle_connection(
    mut stream: TcpStream,
    store: Arc<Store>,
    wal: Arc<Wal>,
    repl_tx: Option<ReplStream>,
    read_only: bool,
) -> std::io::Result<()> {
    let mut buf = BytesMut::with_capacity(4096);
    loop {
        match parse(&mut buf) {
            Ok(value) => {
                let response = match parse_command(value) {
                    Ok(cmd) => {
                        if read_only && WalRecord::from_command(&cmd).is_some() {
                            RespValue::error("READONLY You can't write against a read only replica.")
                        } else {
                            if let Some(rec) = WalRecord::from_command(&cmd) {
                                wal.append(&rec).await?;
                                if let Some(tx) = &repl_tx { let _ = tx.send(rec); }
                            }
                            store.apply(&cmd).await
                        }
                    }
                    Err(e) => RespValue::error(format!("ERR {}", e)),
                };
                stream.write_all(&encode(&response)).await?;
            }
            Err(RespError::Incomplete) => {
                let n = stream.read_buf(&mut buf).await?;
                if n == 0 { return Ok(()); }
            }
            Err(other) => {
                let err = RespValue::error(format!("ERR {}", other));
                stream.write_all(&encode(&err)).await?;
                buf.clear();
            }
        }
    }
}
