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
use crate::live::{self, LiveSnapshot};

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

#[tauri::command]
pub async fn start(app: AppHandle, handle: isize) -> Result<LiveSnapshot, CommandError> {
    tauri::async_runtime::spawn_blocking(move || live::start(&app, handle))
        .await
        .map_err(|error| CommandError::Internal(error.to_string()))?
}

#[tauri::command]
pub async fn stop(app: AppHandle) -> Result<(), CommandError> {
    tauri::async_runtime::spawn_blocking(move || live::stop(&app))
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
