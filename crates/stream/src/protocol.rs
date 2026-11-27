//! Wire format shared with the player page (`src/shared/protocol.ts`).
//!
//! Binary messages carry media: a 4-byte header followed by the payload. The
//! header is 4 bytes so that the audio payload stays aligned for a
//! `Float32Array` view on the receiving side. Text messages carry [`Status`]
//! as JSON.

use bytes::{BufMut, Bytes, BytesMut};
use serde::Serialize;

pub const HEADER_LEN: usize = 4;

pub const SAMPLE_RATE: u32 = 48_000;
pub const CHANNELS: u16 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PacketKind {
    /// One JPEG image.
    Video = 1,
    /// Interleaved stereo `f32` little-endian samples at [`SAMPLE_RATE`].
    Audio = 2,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Status {
    pub live: bool,
    pub title: String,
}

#[must_use]
pub fn video_packet(jpeg: &[u8]) -> Bytes {
    let mut packet = header(PacketKind::Video, jpeg.len());
    packet.put_slice(jpeg);

    packet.freeze()
}

#[must_use]
pub fn audio_packet(samples: &[f32]) -> Bytes {
    let mut packet = header(PacketKind::Audio, size_of_val(samples));
    for sample in samples {
        packet.put_f32_le(*sample);
    }

    packet.freeze()
}

fn header(kind: PacketKind, payload_len: usize) -> BytesMut {
    let mut packet = BytesMut::with_capacity(HEADER_LEN + payload_len);
    packet.put_u8(kind as u8);
    packet.put_bytes(0, HEADER_LEN - 1);

    packet
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_packet_prefixes_jpeg_with_header() {
        let packet = video_packet(&[0xFF, 0xD8]);

        assert_eq!(&packet[..], &[1, 0, 0, 0, 0xFF, 0xD8]);
    }

    #[test]
    fn audio_packet_encodes_samples_as_little_endian_f32() {
        let packet = audio_packet(&[1.0, -0.5]);

        assert_eq!(packet.len(), HEADER_LEN + 8);
        assert_eq!(packet[0], PacketKind::Audio as u8);
        assert_eq!(&packet[4..8], &1.0_f32.to_le_bytes());
        assert_eq!(&packet[8..12], &(-0.5_f32).to_le_bytes());
    }

    #[test]
    fn status_serializes_as_json() {
        let status = Status {
            live: true,
            title: "Demo".into(),
        };

        let json = serde_json::to_string(&status).unwrap();

        assert_eq!(json, r#"{"live":true,"title":"Demo"}"#);
    }
}
