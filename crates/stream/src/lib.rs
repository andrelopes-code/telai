//! Streams captured frames and audio to a browser page on the same machine.

mod encoder;
mod hub;
mod pool;
pub mod protocol;
mod server;

pub use hub::Hub;
pub use pool::EncoderPool;
pub use server::{Asset, AssetSource, DEFAULT_PORT, Server};
