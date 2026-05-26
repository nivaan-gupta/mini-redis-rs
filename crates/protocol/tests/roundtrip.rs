use bytes::{Bytes, BytesMut};
use proptest::prelude::*;
use protocol::{encode, parse, RespValue};

fn arb_resp_value() -> impl Strategy<Value = RespValue> {
    let leaf = prop_oneof![
        any::<String>()
            .prop_filter("no CR/LF in simple", |s| !s.contains('\r')
                && !s.contains('\n'))
            .prop_map(RespValue::SimpleString),
        any::<String>()
            .prop_filter("no CR/LF in error", |s| !s.contains('\r')
                && !s.contains('\n'))
            .prop_map(RespValue::Error),
        any::<i64>().prop_map(RespValue::Integer),
        any::<Vec<u8>>().prop_map(|v| RespValue::BulkString(Some(Bytes::from(v)))),
        Just(RespValue::BulkString(None)),
    ];
    leaf.prop_recursive(3, 16, 4, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..4).prop_map(|v| RespValue::Array(Some(v))),
            Just(RespValue::Array(None)),
        ]
    })
}

proptest! {
    #[test]
    fn encode_then_parse_roundtrip(val in arb_resp_value()) {
        let bytes = encode(&val);
        let mut buf = BytesMut::from(&bytes[..]);
        let parsed = parse(&mut buf).expect("parse must succeed");
        prop_assert_eq!(parsed, val);
        prop_assert!(buf.is_empty(), "all bytes consumed");
    }
}
