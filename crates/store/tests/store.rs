use bytes::Bytes;
use command::{Command, SetCond};
use protocol::RespValue;
use std::time::Duration;
use store::Store;

fn b(s: &'static [u8]) -> Bytes {
    Bytes::from_static(s)
}

#[tokio::test]
async fn set_then_get() {
    let s = Store::new();
    s.apply(&Command::Set {
        key: b(b"k"),
        value: b(b"v"),
        ttl: None,
        cond: SetCond::Always,
    })
    .await;
    assert_eq!(
        s.apply(&Command::Get(b(b"k"))).await,
        RespValue::BulkString(Some(b(b"v")))
    );
}

#[tokio::test]
async fn get_missing_returns_null_bulk() {
    let s = Store::new();
    assert_eq!(
        s.apply(&Command::Get(b(b"nope"))).await,
        RespValue::null_bulk()
    );
}

#[tokio::test]
async fn del_count() {
    let s = Store::new();
    s.apply(&Command::Set {
        key: b(b"a"),
        value: b(b"1"),
        ttl: None,
        cond: SetCond::Always,
    })
    .await;
    s.apply(&Command::Set {
        key: b(b"b"),
        value: b(b"2"),
        ttl: None,
        cond: SetCond::Always,
    })
    .await;
    assert_eq!(
        s.apply(&Command::Del(vec![b(b"a"), b(b"missing"), b(b"b")]))
            .await,
        RespValue::Integer(2)
    );
}

#[tokio::test]
async fn incr_creates_then_increments() {
    let s = Store::new();
    assert_eq!(
        s.apply(&Command::Incr(b(b"n"))).await,
        RespValue::Integer(1)
    );
    assert_eq!(
        s.apply(&Command::Incr(b(b"n"))).await,
        RespValue::Integer(2)
    );
    assert_eq!(
        s.apply(&Command::Decr(b(b"n"))).await,
        RespValue::Integer(1)
    );
}

#[tokio::test]
async fn incr_on_non_integer_errors() {
    let s = Store::new();
    s.apply(&Command::Set {
        key: b(b"k"),
        value: b(b"abc"),
        ttl: None,
        cond: SetCond::Always,
    })
    .await;
    match s.apply(&Command::Incr(b(b"k"))).await {
        RespValue::Error(_) => {}
        other => panic!("expected error, got {:?}", other),
    }
}

#[tokio::test]
async fn ttl_expires_value() {
    let s = Store::new();
    s.apply(&Command::Set {
        key: b(b"k"),
        value: b(b"v"),
        ttl: Some(Duration::from_secs(1)),
        cond: SetCond::Always,
    })
    .await;
    assert_eq!(
        s.apply(&Command::Get(b(b"k"))).await,
        RespValue::BulkString(Some(b(b"v")))
    );
    tokio::time::sleep(Duration::from_millis(1100)).await;
    assert_eq!(
        s.apply(&Command::Get(b(b"k"))).await,
        RespValue::null_bulk()
    );
}

#[tokio::test]
async fn nx_only_sets_if_absent() {
    let s = Store::new();
    assert_eq!(
        s.apply(&Command::Set {
            key: b(b"k"),
            value: b(b"first"),
            ttl: None,
            cond: SetCond::IfAbsent,
        })
        .await,
        RespValue::ok()
    );
    assert_eq!(
        s.apply(&Command::Set {
            key: b(b"k"),
            value: b(b"second"),
            ttl: None,
            cond: SetCond::IfAbsent,
        })
        .await,
        RespValue::null_bulk()
    );
    assert_eq!(
        s.apply(&Command::Get(b(b"k"))).await,
        RespValue::BulkString(Some(b(b"first")))
    );
}

#[tokio::test]
async fn keys_glob_star() {
    let s = Store::new();
    for k in [b"user:1" as &[u8], b"user:2", b"order:9"] {
        s.apply(&Command::Set {
            key: Bytes::copy_from_slice(k),
            value: b(b"x"),
            ttl: None,
            cond: SetCond::Always,
        })
        .await;
    }
    let res = s.apply(&Command::Keys(Bytes::from_static(b"user:*"))).await;
    match res {
        RespValue::Array(Some(items)) => assert_eq!(items.len(), 2),
        other => panic!("expected array, got {:?}", other),
    }
}
