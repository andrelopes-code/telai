use bytes::Bytes;
use fast_image_resize::images::{Image, ImageRef};
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use jpeg_encoder::{ColorType, Encoder, SamplingFactor};

use crate::protocol;

/// Frames taller than this are scaled down before encoding, which keeps the
/// encoding time of large windows within the frame budget.
const MAX_HEIGHT: u32 = 1080;

/// Chroma is subsampled 4:2:0 like the video sources themselves, so quality
/// goes to luma detail.
const QUALITY: u8 = 90;

const BYTES_PER_PIXEL: usize = 4;

#[derive(Debug, thiserror::Error)]
pub enum EncodeError {
    #[error("frame buffer holds {actual} bytes, expected {expected}")]
    BufferSize { expected: usize, actual: usize },
    #[error("frame of {width}x{height} exceeds the JPEG size limit")]
    TooLarge { width: u32, height: u32 },
    #[error(transparent)]
    Image(#[from] fast_image_resize::ImageBufferError),
    #[error(transparent)]
    Resize(#[from] fast_image_resize::ResizeError),
    #[error(transparent)]
    Jpeg(#[from] jpeg_encoder::EncodingError),
}

/// A captured frame: tightly packed BGRA rows.
#[derive(Debug, Clone, Copy)]
pub struct Frame<'a> {
    pub width: u32,
    pub height: u32,
    pub bgra: &'a [u8],
}

/// Turns BGRA frames into video packets. Reuses its buffers across frames.
pub struct FrameEncoder {
    resizer: Resizer,
    scaled: Vec<u8>,
    jpeg: Vec<u8>,
}

impl FrameEncoder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            resizer: Resizer::new(),
            scaled: Vec::new(),
            jpeg: Vec::new(),
        }
    }

    pub fn encode(&mut self, frame: Frame<'_>) -> Result<Bytes, EncodeError> {
        let expected = frame.width as usize * frame.height as usize * BYTES_PER_PIXEL;
        if frame.bgra.len() != expected {
            return Err(EncodeError::BufferSize {
                expected,
                actual: frame.bgra.len(),
            });
        }

        let (width, height) = scaled_size(frame.width, frame.height);
        let pixels = if height == frame.height {
            frame.bgra
        } else {
            self.scale(frame, width, height)?;
            &self.scaled
        };

        let too_large = || EncodeError::TooLarge { width, height };
        let jpeg_width = u16::try_from(width).map_err(|_| too_large())?;
        let jpeg_height = u16::try_from(height).map_err(|_| too_large())?;

        self.jpeg.clear();
        let mut encoder = Encoder::new(&mut self.jpeg, QUALITY);
        encoder.set_sampling_factor(SamplingFactor::F_2_2);
        encoder.encode(pixels, jpeg_width, jpeg_height, ColorType::Bgra)?;

        Ok(protocol::video_packet(&self.jpeg))
    }

    fn scale(&mut self, frame: Frame<'_>, width: u32, height: u32) -> Result<(), EncodeError> {
        let source = ImageRef::new(frame.width, frame.height, frame.bgra, PixelType::U8x4)?;

        self.scaled
            .resize(width as usize * height as usize * BYTES_PER_PIXEL, 0);
        let mut target = Image::from_slice_u8(width, height, &mut self.scaled, PixelType::U8x4)?;

        let options = ResizeOptions::new()
            .resize_alg(ResizeAlg::Convolution(FilterType::Bilinear))
            .use_alpha(false);
        self.resizer.resize(&source, &mut target, &options)?;

        Ok(())
    }
}

impl Default for FrameEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Keeps the aspect ratio and an even width, which JPEG 4:2:0 encodes without
/// a padded edge column.
fn scaled_size(width: u32, height: u32) -> (u32, u32) {
    if height <= MAX_HEIGHT {
        return (width, height);
    }

    let scaled_width = u64::from(width) * u64::from(MAX_HEIGHT) / u64::from(height);
    let even_width = u32::try_from(scaled_width).unwrap_or(u32::MAX) & !1;

    (even_width.max(2), MAX_HEIGHT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{HEADER_LEN, PacketKind};

    #[test]
    fn encode_produces_a_video_packet_with_a_jpeg_payload() {
        let bgra = gradient(64, 48);
        let mut encoder = FrameEncoder::new();

        let packet = encoder
            .encode(Frame {
                width: 64,
                height: 48,
                bgra: &bgra,
            })
            .unwrap();

        assert_eq!(packet[0], PacketKind::Video as u8);
        assert_eq!(&packet[HEADER_LEN..HEADER_LEN + 2], &[0xFF, 0xD8]);
        assert_eq!(jpeg_size(&packet[HEADER_LEN..]), (64, 48));
    }

    #[test]
    fn encode_scales_frames_above_1080p_down_to_1080p() {
        let bgra = gradient(3840, 2160);
        let mut encoder = FrameEncoder::new();

        let packet = encoder
            .encode(Frame {
                width: 3840,
                height: 2160,
                bgra: &bgra,
            })
            .unwrap();

        assert_eq!(jpeg_size(&packet[HEADER_LEN..]), (1920, 1080));
    }

    #[test]
    fn encode_rejects_a_buffer_that_does_not_match_the_size() {
        let mut encoder = FrameEncoder::new();

        let result = encoder.encode(Frame {
            width: 4,
            height: 4,
            bgra: &[0; 10],
        });

        assert!(matches!(
            result,
            Err(EncodeError::BufferSize {
                expected: 64,
                actual: 10
            })
        ));
    }

    #[test]
    fn scaled_size_keeps_aspect_ratio_with_an_even_width() {
        assert_eq!(scaled_size(1280, 720), (1280, 720));
        assert_eq!(scaled_size(2560, 1440), (1920, 1080));
        assert_eq!(scaled_size(3441, 1441), (2578, 1080));
    }

    fn gradient(width: u32, height: u32) -> Vec<u8> {
        (0..width * height)
            .flat_map(|index| {
                let x = u8::try_from(index % width % 256).unwrap();
                let y = u8::try_from(index / width % 256).unwrap();
                [x, y, x ^ y, 255]
            })
            .collect()
    }

    /// Reads the dimensions from the first start-of-frame marker.
    fn jpeg_size(jpeg: &[u8]) -> (u16, u16) {
        let marker = jpeg
            .windows(2)
            .position(|pair| pair == [0xFF, 0xC0])
            .unwrap();
        let height = u16::from_be_bytes([jpeg[marker + 5], jpeg[marker + 6]]);
        let width = u16::from_be_bytes([jpeg[marker + 7], jpeg[marker + 8]]);

        (width, height)
    }
}
