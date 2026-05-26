use bytes::{Bytes, BytesMut};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use protocol::{encode, parse, RespValue};

fn bench_encode(c: &mut Criterion) {
    let v = RespValue::Array(Some(vec![
        RespValue::BulkString(Some(Bytes::from_static(b"SET"))),
        RespValue::BulkString(Some(Bytes::from_static(b"user:42:name"))),
        RespValue::BulkString(Some(Bytes::from_static(b"Nivaan Gupta"))),
    ]));
    c.bench_function("encode_set", |b| {
        b.iter(|| {
            let _ = black_box(encode(black_box(&v)));
        })
    });
}

fn bench_decode(c: &mut Criterion) {
    let bytes = encode(&RespValue::Array(Some(vec![
        RespValue::BulkString(Some(Bytes::from_static(b"SET"))),
        RespValue::BulkString(Some(Bytes::from_static(b"k"))),
        RespValue::BulkString(Some(Bytes::from_static(b"v"))),
    ])));
    c.bench_function("decode_set", |b| {
        b.iter(|| {
            let mut buf = BytesMut::from(&bytes[..]);
            let _ = black_box(parse(&mut buf));
        })
    });
}

criterion_group!(benches, bench_encode, bench_decode);
criterion_main!(benches);
