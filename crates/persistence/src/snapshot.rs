use bytes::Bytes;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use store::Entry;
use tokio::fs;
use tokio::io::AsyncWriteExt;

pub struct Snapshot {
    path: PathBuf,
}

impl Snapshot {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub async fn write(&self, data: &HashMap<Bytes, Entry>) -> std::io::Result<()> {
        let tmp = self.path.with_extension("rdb.tmp");
        let serializable: HashMap<Vec<u8>, Entry> =
            data.iter().map(|(k, v)| (k.to_vec(), v.clone())).collect();
        let bytes = bincode::serialize(&serializable).expect("snapshot serializes");
        let mut f = fs::File::create(&tmp).await?;
        f.write_all(&bytes).await?;
        f.sync_data().await?;
        fs::rename(&tmp, &self.path).await?; // atomic replace
        Ok(())
    }

    pub async fn read(&self) -> std::io::Result<Option<HashMap<Bytes, Entry>>> {
        let bytes = match fs::read(&self.path).await {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        let raw: HashMap<Vec<u8>, Entry> = bincode::deserialize(&bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(Some(
            raw.into_iter().map(|(k, v)| (Bytes::from(k), v)).collect(),
        ))
    }
}
