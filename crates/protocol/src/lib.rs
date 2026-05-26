//! RESP2 wire protocol — parsing and encoding.

mod error;
mod value;

pub use error::{RespError, Result};
pub use value::RespValue;
