#![allow(clippy::large_enum_variant)]

pub use tokio_util::codec::BytesCodec;

pub use self::codec::{Codec, CodecLog};
pub use self::connection::Connection;

pub mod codec;
pub mod connection;
pub use proto;
pub mod casemapping;
pub mod string;
