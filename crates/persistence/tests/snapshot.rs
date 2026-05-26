use bytes::Bytes;
use persistence::Snapshot;
use std::collections::HashMap;
use store::Entry;
use tempfile::tempdir;

#[tokio::test]
async fn round_trips_data() {
    let dir = tempdir().unwrap();
    let snap = Snapshot::new(dir.path().join("snap.rdb"));
    let mut data = HashMap::new();
    data.insert(
        Bytes::from_static(b"k1"),
        Entry::new(Bytes::from_static(b"v1"), None),
    );
    data.insert(
        Bytes::from_static(b"k2"),
        Entry::new(Bytes::from_static(b"v2"), None),
    );

    snap.write(&data).await.unwrap();
    let back = snap.read().await.unwrap().unwrap();
    assert_eq!(back.len(), 2);
    assert_eq!(
        back.get(&Bytes::from_static(b"k1")).unwrap().value,
        Bytes::from_static(b"v1")
    );
}

#[tokio::test]
async fn missing_file_returns_none() {
    let dir = tempdir().unwrap();
    let snap = Snapshot::new(dir.path().join("nope.rdb"));
    assert!(snap.read().await.unwrap().is_none());
}
