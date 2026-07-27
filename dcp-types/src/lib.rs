pub mod automation;
pub mod capabilities;
pub mod context;
pub mod events;
pub mod platform;
pub mod protocol;
pub mod vision;

#[cfg(test)]
mod tests;

pub use automation::*;
pub use capabilities::*;
pub use context::*;
pub use events::*;
pub use platform::*;
pub use protocol::*;
pub use vision::*;

/// Protocol version.
pub const PROTOCOL_VERSION: &str = "0.1.0";

/// Magic bytes identifying DCP frames (future-proofing).
pub const FRAME_MAGIC: &[u8; 2] = b"DC";
