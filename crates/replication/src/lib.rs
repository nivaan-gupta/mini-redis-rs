//! Replication: leader and replica coordination.

pub mod leader;
pub mod follower;

pub use leader::{channel, broadcast_command, run_replication_listener, ReplStream};
pub use follower::run_replica;
