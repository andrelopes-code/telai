use std::time::{Duration, Instant};

use windows::Win32::UI::WindowsAndMessaging::IsWindow;
use windows_capture::capture::{CaptureControl, Context, GraphicsCaptureApiHandler};
use windows_capture::frame::Frame as CapturedFrame;
use windows_capture::graphics_capture_api::{GraphicsCaptureApi, InternalCaptureControl};
use windows_capture::settings::{
    ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
    MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
};
use windows_capture::window::Window;

use crate::Error;
use crate::window::to_hwnd;

/// 30 fps keeps JPEG encoding of 1080p frames within the two encoder threads.
const FRAME_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / 30);

const BYTES_PER_PIXEL: usize = 4;

/// A captured BGRA frame. Rows may be padded beyond `width` pixels.
pub struct Frame<'a> {
    pub width: u32,
    pub height: u32,
    row_pitch: usize,
    data: &'a [u8],
}

impl Frame<'_> {
    /// Replaces the contents of `target` with tightly packed BGRA rows.
    pub fn copy_to(&self, target: &mut Vec<u8>) {
        let row_len = self.width as usize * BYTES_PER_PIXEL;
        target.clear();
        target.reserve(row_len * self.height as usize);

        for row in self.data.chunks(self.row_pitch).take(self.height as usize) {
            target.extend_from_slice(&row[..row_len]);
        }
    }
}

/// Receives the frames of a [`VideoCapture`] on its capture thread.
pub trait VideoSink: Send + 'static {
    fn frame(&mut self, frame: &Frame<'_>);

    /// The window was closed. No frame follows.
    fn closed(&mut self);
}

/// Captures one window until stopped or until the window closes.
pub struct VideoCapture {
    control: Option<CaptureControl<Handler, windows_capture::frame::Error>>,
}

impl VideoCapture {
    pub fn start(handle: isize, sink: impl VideoSink) -> Result<Self, Error> {
        let hwnd = to_hwnd(handle);
        if !unsafe { IsWindow(Some(hwnd)).as_bool() } {
            return Err(Error::Video("the window no longer exists".into()));
        }

        let sink: Box<dyn VideoSink> = Box::new(sink);
        let settings = Settings::new(
            Window::from_raw_hwnd(hwnd.0),
            supported_or_default(
                GraphicsCaptureApi::is_cursor_settings_supported(),
                CursorCaptureSettings::WithoutCursor,
                CursorCaptureSettings::Default,
            ),
            supported_or_default(
                GraphicsCaptureApi::is_border_settings_supported(),
                DrawBorderSettings::WithoutBorder,
                DrawBorderSettings::Default,
            ),
            SecondaryWindowSettings::Default,
            supported_or_default(
                GraphicsCaptureApi::is_minimum_update_interval_supported(),
                MinimumUpdateIntervalSettings::Custom(FRAME_INTERVAL),
                MinimumUpdateIntervalSettings::Default,
            ),
            DirtyRegionSettings::Default,
            ColorFormat::Bgra8,
            sink,
        );

        let control = Handler::start_free_threaded(settings)
            .map_err(|error| Error::Video(error.to_string()))?;

        Ok(Self {
            control: Some(control),
        })
    }

    /// Whether the capture ended on its own, because the window closed or
    /// the capture failed.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.control
            .as_ref()
            .is_none_or(CaptureControl::is_finished)
    }

    pub fn stop(mut self) {
        self.stop_capture();
    }

    fn stop_capture(&mut self) {
        let Some(control) = self.control.take() else {
            return;
        };

        // A finished capture thread no longer runs the message loop that a
        // stop request is posted to.
        let result = if control.is_finished() {
            control.wait()
        } else {
            control.stop()
        };
        if let Err(error) = result {
            tracing::warn!(%error, "window capture ended with an error");
        }
    }
}

impl Drop for VideoCapture {
    fn drop(&mut self) {
        self.stop_capture();
    }
}

/// Settings that the running Windows version lacks fall back to the default,
/// instead of failing the whole capture.
fn supported_or_default<T>(
    supported: Result<bool, windows_capture::graphics_capture_api::Error>,
    wanted: T,
    default: T,
) -> T {
    if supported.unwrap_or(false) {
        wanted
    } else {
        default
    }
}

struct Handler {
    sink: Box<dyn VideoSink>,
    last_frame: Option<Instant>,
}

impl GraphicsCaptureApiHandler for Handler {
    type Flags = Box<dyn VideoSink>;
    type Error = windows_capture::frame::Error;

    fn new(context: Context<Self::Flags>) -> Result<Self, Self::Error> {
        Ok(Self {
            sink: context.flags,
            last_frame: None,
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut CapturedFrame,
        _control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        // Windows versions without a minimum update interval deliver frames
        // at the display refresh rate.
        let now = Instant::now();
        if self
            .last_frame
            .is_some_and(|last| now.duration_since(last) < FRAME_INTERVAL)
        {
            return Ok(());
        }
        self.last_frame = Some(now);

        // An error here would end the capture; a skipped frame is replaced by
        // the next one 33 ms later.
        let mut buffer = match frame.buffer_without_title_bar() {
            Ok(buffer) => buffer,
            Err(error) => {
                tracing::debug!(%error, "skipping a frame that could not be read");
                return Ok(());
            }
        };
        let frame = Frame {
            width: buffer.width(),
            height: buffer.height(),
            row_pitch: buffer.row_pitch() as usize,
            data: buffer.as_raw_buffer(),
        };
        self.sink.frame(&frame);

        Ok(())
    }

    fn on_closed(&mut self) -> Result<(), Self::Error> {
        self.sink.closed();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_to_drops_row_padding() {
        let data = [1, 1, 1, 1, 9, 9, 2, 2, 2, 2, 9, 9];
        let frame = Frame {
            width: 1,
            height: 2,
            row_pitch: 6,
            data: &data,
        };
        let mut target = vec![7; 32];

        frame.copy_to(&mut target);

        assert_eq!(target, [1, 1, 1, 1, 2, 2, 2, 2]);
    }
}
