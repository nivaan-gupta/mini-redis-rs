//! Replication: leader and replica coordination.

pub mod follower;
pub mod leader;

pub use follower::run_replica;
pub use leader::{broadcast_command, channel, run_replication_listener, ReplStream};
