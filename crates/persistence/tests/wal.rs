use persistence::{Wal, WalRecord};
use tempfile::tempdir;

#[tokio::test]
async fn append_then_replay_round_trip() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("wal.log");
    let wal = Wal::open(&path, false).await.unwrap();

    let records = vec![
        WalRecord::Set {
            key: b"k1".to_vec(),
            value: b"v1".to_vec(),
            ttl_secs: None,
        },
        WalRecord::Set {
            key: b"k2".to_vec(),
            value: b"v2".to_vec(),
            ttl_secs: Some(60),
        },
        WalRecord::Del {
            keys: vec![b"k1".to_vec()],
        },
        WalRecord::Incr {
            key: b"counter".to_vec(),
            delta: 1,
        },
    ];
    for r in &records {
        wal.append(r).await.unwrap();
    }

    let replayed = wal.replay().await.unwrap();
    assert_eq!(replayed, records);
}

#[tokio::test]
async fn truncated_wal_stops_at_last_complete_record() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("wal.log");
    let wal = Wal::open(&path, false).await.unwrap();

    wal.append(&WalRecord::Set {
        key: b"k".to_vec(),
        value: b"v".to_vec(),
        ttl_secs: None,
    })
    .await
    .unwrap();

    // Append a length prefix but no payload (simulate crash mid-write).
    use tokio::fs::OpenOptions;
    use tokio::io::AsyncWriteExt;
    let mut f = OpenOptions::new().append(true).open(&path).await.unwrap();
    f.write_all(&[0, 0, 0, 42]).await.unwrap(); // claims 42-byte record
    f.flush().await.unwrap();
    drop(f);

    let replayed = wal.replay().await.unwrap();
    assert_eq!(replayed.len(), 1);
}
