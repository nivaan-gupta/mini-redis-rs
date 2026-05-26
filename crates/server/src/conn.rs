use bytes::BytesMut;
use command::parse_command;
use protocol::{encode, parse, RespError, RespValue};
use std::sync::Arc;
use store::Store;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub async fn handle_connection(mut stream: TcpStream, store: Arc<Store>) -> std::io::Result<()> {
    let mut buf = BytesMut::with_capacity(4096);
    loop {
        match parse(&mut buf) {
            Ok(value) => {
                let response = match parse_command(value) {
                    Ok(cmd) => store.apply(&cmd).await,
                    Err(e) => RespValue::error(format!("ERR {}", e)),
                };
                stream.write_all(&encode(&response)).await?;
            }
            Err(RespError::Incomplete) => {
                let n = stream.read_buf(&mut buf).await?;
                if n == 0 {
                    return Ok(()); // client disconnected
                }
            }
            Err(other) => {
                let err = RespValue::error(format!("ERR {}", other));
                stream.write_all(&encode(&err)).await?;
                buf.clear();
            }
        }
    }
}
