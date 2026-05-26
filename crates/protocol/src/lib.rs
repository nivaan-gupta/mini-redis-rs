//! RESP2 wire protocol — parsing and encoding.

mod decode;
mod encode;
mod error;
mod value;

pub use decode::parse;
pub use encode::encode;
pub use error::{RespError, Result};
pub use value::RespValue;
