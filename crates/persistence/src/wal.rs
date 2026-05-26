//! Append-only write-ahead log. Each record:
//! [u32 length BE][bincode-encoded WalRecord]

use crate::record::WalRecord;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufReader, SeekFrom};
use tokio::sync::Mutex;

pub struct Wal {
    file: Arc<Mutex<File>>,
    path: PathBuf,
    fsync_every: bool,
}

impl Wal {
    pub async fn open(path: impl AsRef<Path>, fsync_every: bool) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)
            .await?;
        Ok(Self {
            file: Arc::new(Mutex::new(file)),
            path,
            fsync_every,
        })
    }

    pub async fn append(&self, record: &WalRecord) -> std::io::Result<()> {
        let payload = bincode::serialize(record).expect("WAL record serializable");
        let len = payload.len() as u32;
        let mut guard = self.file.lock().await;
        guard.write_all(&len.to_be_bytes()).await?;
        guard.write_all(&payload).await?;
        if self.fsync_every {
            guard.sync_data().await?;
        } else {
            guard.flush().await?;
        }
        Ok(())
    }

    pub async fn replay(&self) -> std::io::Result<Vec<WalRecord>> {
        let f = File::open(&self.path).await?;
        let mut reader = BufReader::new(f);
        let mut out = Vec::new();
        loop {
            let mut len_buf = [0u8; 4];
            match reader.read_exact(&mut len_buf).await {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e),
            }
            let len = u32::from_be_bytes(len_buf) as usize;
            let mut payload = vec![0u8; len];
            match reader.read_exact(&mut payload).await {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    tracing::warn!("WAL truncated mid-record; stopping replay");
                    break;
                }
                Err(e) => return Err(e),
            }
            match bincode::deserialize(&payload) {
                Ok(r) => out.push(r),
                Err(e) => {
                    tracing::warn!(error = %e, "WAL record corrupt; stopping replay");
                    break;
                }
            }
        }
        Ok(out)
    }

    pub async fn truncate(&self) -> std::io::Result<()> {
        let mut guard = self.file.lock().await;
        guard.set_len(0).await?;
        guard.seek(SeekFrom::Start(0)).await?;
        Ok(())
    }
}
