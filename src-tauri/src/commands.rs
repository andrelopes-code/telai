#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri passes command arguments by value"
)]

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Serialize;
use std::sync::PoisonError;

use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::app::AppState;
use crate::live::{self, LiveSnapshot, SourceRef};

/// Failure of a command, with a code that the window translates.
#[derive(Debug, Serialize)]
#[serde(tag = "code", content = "detail", rename_all = "camelCase")]
pub enum CommandError {
    /// The window closed between listing and starting.
    WindowGone,
    VideoUnavailable(String),
    AudioUnavailable(String),
    Internal(String),
}

impl CommandError {
    pub fn video(error: telai_capture::Error) -> Self {
        Self::VideoUnavailable(error.to_string())
    }

    pub fn audio(error: telai_capture::Error) -> Self {
        Self::AudioUnavailable(error.to_string())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    url: String,
    server_error: Option<String>,
    /// "acrylic" when Windows paints the Acrylic material behind the window,
    /// "solid" when the window must draw an opaque background.
    backdrop: &'static str,
    live: Option<LiveSnapshot>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowView {
    handle: isize,
    title: String,
    process_name: String,
    icon: Option<String>,
    thumbnail: Option<String>,
    playing_audio: bool,
}

#[tauri::command]
pub fn status(app: AppHandle, state: State<'_, AppState>) -> AppStatus {
    AppStatus {
        url: state.url.clone(),
        server_error: state
            .server_error
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone(),
        backdrop: if telai_capture::supports_acrylic() {
            "acrylic"
        } else {
            "solid"
        },
        live: live::snapshot(&app),
    }
}

#[tauri::command]
pub async fn list_windows() -> Result<Vec<WindowView>, CommandError> {
    let windows = tauri::async_runtime::spawn_blocking(telai_capture::list_windows)
        .await
        .map_err(|error| CommandError::Internal(error.to_string()))?
        .map_err(CommandError::video)?;

    let views = windows
        .into_iter()
        .map(|window| WindowView {
            handle: window.handle,
            icon: window.icon_png.as_deref().map(icon_data_url),
            thumbnail: window.thumbnail_jpeg.as_deref().map(jpeg_data_url),
            title: window.title,
            process_name: window.process_name,
            playing_audio: window.playing_audio,
        })
        .collect();

    Ok(views)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorView {
    handle: isize,
    number: usize,
    name: String,
    width: u32,
    height: u32,
    primary: bool,
    thumbnail: Option<String>,
}

#[tauri::command]
pub async fn list_monitors() -> Result<Vec<MonitorView>, CommandError> {
    let monitors = tauri::async_runtime::spawn_blocking(telai_capture::list_monitors)
        .await
        .map_err(|error| CommandError::Internal(error.to_string()))?
        .map_err(CommandError::video)?;

    let views = monitors
        .into_iter()
        .map(|monitor| MonitorView {
            handle: monitor.handle,
            number: monitor.number,
            thumbnail: monitor.thumbnail_jpeg.as_deref().map(jpeg_data_url),
            name: monitor.name,
            width: monitor.width,
            height: monitor.height,
            primary: monitor.primary,
        })
        .collect();

    Ok(views)
}

#[tauri::command]
pub async fn start(app: AppHandle, source: SourceRef) -> Result<LiveSnapshot, CommandError> {
    tauri::async_runtime::spawn_blocking(move || live::start(&app, source))
        .await
        .map_err(|error| CommandError::Internal(error.to_string()))?
}

#[tauri::command]
pub async fn stop(app: AppHandle) -> Result<(), CommandError> {
    tauri::async_runtime::spawn_blocking(move || live::stop(&app))
        .await
        .map_err(|error| CommandError::Internal(error.to_string()))
}

/// Includes or leaves out the app `name` in screen streams, at `volume`
/// from 0 to 1.
#[tauri::command]
pub async fn set_audio_app(
    app: AppHandle,
    name: String,
    included: bool,
    volume: f32,
) -> Result<(), CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        live::set_audio_app(&app, &name, included, volume);
    })
    .await
    .map_err(|error| CommandError::Internal(error.to_string()))
}

#[tauri::command]
pub fn open_player(app: AppHandle, state: State<'_, AppState>) -> Result<(), CommandError> {
    app.opener()
        .open_url(&state.url, None::<&str>)
        .map_err(|error| CommandError::Internal(error.to_string()))
}

/// Tries to bind the player port again, after the program that held it quit.
#[tauri::command]
pub async fn retry_server(app: AppHandle) -> Result<AppStatus, CommandError> {
    let status_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || crate::app::start_server(&app))
        .await
        .map_err(|error| CommandError::Internal(error.to_string()))?;

    Ok(status(status_app.clone(), status_app.state::<AppState>()))
}

pub fn icon_data_url(png: &[u8]) -> String {
    format!("data:image/png;base64,{}", STANDARD.encode(png))
}

fn jpeg_data_url(jpeg: &[u8]) -> String {
    format!("data:image/jpeg;base64,{}", STANDARD.encode(jpeg))
}
