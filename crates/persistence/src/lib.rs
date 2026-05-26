//! Persistence: WAL + snapshot + recovery.

pub mod record;
pub mod wal;

pub use record::WalRecord;
pub use wal::Wal;
